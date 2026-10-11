//! ModernBERT encoder and Laya decision heads, in candle.
//!
//! Architecture follows upstream Laya and Hugging Face ModernBERT. Four
//! details are easy to get wrong and are called out where they occur:
//!
//! 1. The encoder and scoring head use **exact** (erf) GELU, so `gelu_erf` —
//!    candle's `gelu` is the tanh approximation.
//! 2. The decision-head feed-forward uses **ReLU**, because upstream builds it
//!    from `torch.nn.TransformerEncoderLayer`, whose default activation is ReLU
//!    even though everything around it is GELU.
//! 3. Padded *queries* are allowed to attend to valid keys. Without that a
//!    fully-masked softmax row produces NaN. Padded positions are never used as
//!    keys or pooled outputs, so valid-token results are unchanged.
//! 4. RoPE tables are built in f32 and cast afterwards; building them directly
//!    in f16 loses precision at long positions.

use candle_core::{DType, Device, IndexOp, Result, Tensor, D};
use candle_nn::{
    embedding, layer_norm, layer_norm_no_bias, linear, linear_no_bias, ops::softmax_last_dim,
    Embedding, LayerNorm, LayerNormConfig, Linear, Module, VarBuilder,
};
use std::sync::Arc;

use crate::config::{AgentConfig, EncoderConfig, FULL_ATTENTION};

/// Logit assigned to marker slots that do not correspond to a real option.
const MASKED_LOGIT: f64 = -1e4;

// ---------------------------------------------------------------- rotary

/// Precomputed RoPE tables for one theta.
#[derive(Debug)]
pub struct Rotary {
    cos: Tensor,
    sin: Tensor,
}

impl Rotary {
    fn new(
        head_dim: usize,
        max_positions: usize,
        base: f64,
        dtype: DType,
        device: &Device,
    ) -> Result<Self> {
        // Built in f32 regardless of the model dtype, then cast once.
        let inv_freq: Vec<f32> = (0..head_dim)
            .step_by(2)
            .map(|index| (1.0 / base.powf(index as f64 / head_dim as f64)) as f32)
            .collect();
        let width = inv_freq.len();
        let inv_freq = Tensor::from_vec(inv_freq, (1, width), device)?;
        let positions = Tensor::arange(0u32, max_positions as u32, device)?
            .to_dtype(DType::F32)?
            .reshape((max_positions, 1))?;
        let freqs = positions.matmul(&inv_freq)?;
        Ok(Self {
            cos: freqs.cos()?.to_dtype(dtype)?,
            sin: freqs.sin()?.to_dtype(dtype)?,
        })
    }

    /// Apply rotary embeddings to `(batch, heads, seq, head_dim)` tensors.
    fn apply(&self, q: &Tensor, k: &Tensor, seq_len: usize) -> Result<(Tensor, Tensor)> {
        let cos = self.cos.narrow(0, 0, seq_len)?;
        let sin = self.sin.narrow(0, 0, seq_len)?;
        // `rope` is the half-split (NeoX) form, matching MLX's
        // `fast.rope(..., traditional=False)`. `rope_i` would be the
        // interleaved variant and is *not* what upstream uses.
        let q = candle_nn::rotary_emb::rope(&q.contiguous()?, &cos, &sin)?;
        let k = candle_nn::rotary_emb::rope(&k.contiguous()?, &cos, &sin)?;
        Ok((q, k))
    }
}

// ---------------------------------------------------------------- masks

/// Additive attention masks: `0` where attention is allowed, `-inf` elsewhere.
///
/// Mirrors upstream's boolean masks, which MLX's SDPA turns into exactly this.
#[derive(Debug, Clone)]
pub struct AttentionMasks {
    /// `(batch, 1, 1, seq)` — key validity.
    pub full: Tensor,
    /// `(batch, 1, seq, seq)` — key validity intersected with the local window.
    pub sliding: Tensor,
}

impl AttentionMasks {
    /// `attention_mask` is `(batch, seq)` with 1 for real tokens.
    pub fn build(attention_mask: &Tensor, window: usize, dtype: DType) -> Result<Self> {
        let device = attention_mask.device();
        let (batch, seq_len) = attention_mask.dims2()?;
        let valid = attention_mask.to_dtype(DType::U8)?;

        let neg_inf = f32::NEG_INFINITY as f64;
        let zeros = Tensor::zeros((batch, 1, 1, seq_len), DType::F32, device)?;
        let blocked = Tensor::full(neg_inf as f32, (batch, 1, 1, seq_len), device)?;
        let key_valid = valid.reshape((batch, 1, 1, seq_len))?;
        let full = key_valid.where_cond(&zeros, &blocked)?;

        // Local window: |i - j| <= window / 2, inclusive, as upstream computes it.
        // Bounded from both sides rather than via abs(): candle's Metal backend
        // has no uabs kernel for I64, and this mask is on the unconditional path,
        // so abs() here made every predict fail on Apple GPUs. Staying in I64
        // also avoids an f32 round trip on long sequences.
        let half = (window / 2) as i64;
        let positions = Tensor::arange(0i64, seq_len as i64, device)?;
        let distance = positions
            .reshape((seq_len, 1))?
            .broadcast_sub(&positions.reshape((1, seq_len))?)?;
        let in_window = distance
            .le(half)?
            .mul(&distance.ge(-half)?)?
            .reshape((1, 1, seq_len, seq_len))?;

        // A padded *query* may see every valid key, so no row is fully masked.
        let query_is_pad = valid.reshape((batch, 1, seq_len, 1))?.eq(0u8)?;
        let allowed = in_window
            .broadcast_as((batch, 1, seq_len, seq_len))?
            .maximum(&query_is_pad.broadcast_as((batch, 1, seq_len, seq_len))?)?
            .mul(&key_valid.broadcast_as((batch, 1, seq_len, seq_len))?)?;

        let zeros = Tensor::zeros((batch, 1, seq_len, seq_len), DType::F32, device)?;
        let blocked = Tensor::full(neg_inf as f32, (batch, 1, seq_len, seq_len), device)?;
        let sliding = allowed.where_cond(&zeros, &blocked)?;

        Ok(Self {
            full: full.to_dtype(dtype)?,
            sliding: sliding.to_dtype(dtype)?,
        })
    }

    fn for_kind(&self, kind: &str) -> &Tensor {
        if kind == FULL_ATTENTION {
            &self.full
        } else {
            &self.sliding
        }
    }
}

/// Scaled dot-product attention with an additive mask.
fn attention(q: &Tensor, k: &Tensor, v: &Tensor, scale: f64, mask: &Tensor) -> Result<Tensor> {
    let scores = (q.matmul(&k.transpose(D::Minus2, D::Minus1)?)? * scale)?;
    let scores = scores.broadcast_add(mask)?;
    softmax_last_dim(&scores)?.matmul(v)
}

// ---------------------------------------------------------------- encoder

#[derive(Debug)]
struct Embeddings {
    tok_embeddings: Embedding,
    norm: LayerNorm,
}

impl Embeddings {
    fn load(vb: VarBuilder, cfg: &EncoderConfig) -> Result<Self> {
        Ok(Self {
            tok_embeddings: embedding(cfg.vocab_size, cfg.hidden_size, vb.pp("tok_embeddings"))?,
            norm: norm(cfg.hidden_size, cfg.norm_eps, cfg.norm_bias, vb.pp("norm"))?,
        })
    }

    fn forward(&self, ids: &Tensor) -> Result<Tensor> {
        self.norm.forward(&self.tok_embeddings.forward(ids)?)
    }
}

/// LayerNorm honouring the checkpoint's `norm_bias` flag.
fn norm(size: usize, eps: f64, bias: bool, vb: VarBuilder) -> Result<LayerNorm> {
    if bias {
        layer_norm(
            size,
            LayerNormConfig {
                eps,
                ..Default::default()
            },
            vb,
        )
    } else {
        layer_norm_no_bias(size, eps, vb)
    }
}

/// Linear honouring an optional bias flag.
fn maybe_biased_linear(
    in_dim: usize,
    out_dim: usize,
    bias: bool,
    vb: VarBuilder,
) -> Result<Linear> {
    if bias {
        linear(in_dim, out_dim, vb)
    } else {
        linear_no_bias(in_dim, out_dim, vb)
    }
}

#[derive(Debug)]
struct EncoderAttention {
    wqkv: Linear,
    wo: Linear,
    num_heads: usize,
    head_dim: usize,
    rotary: Arc<Rotary>,
}

impl EncoderAttention {
    fn load(vb: VarBuilder, cfg: &EncoderConfig, rotary: Arc<Rotary>) -> Result<Self> {
        Ok(Self {
            wqkv: maybe_biased_linear(
                cfg.hidden_size,
                3 * cfg.hidden_size,
                cfg.attention_bias,
                vb.pp("Wqkv"),
            )?,
            wo: maybe_biased_linear(
                cfg.hidden_size,
                cfg.hidden_size,
                cfg.attention_bias,
                vb.pp("Wo"),
            )?,
            num_heads: cfg.num_attention_heads,
            head_dim: cfg.head_dim(),
            rotary,
        })
    }

    fn forward(&self, xs: &Tensor, mask: &Tensor) -> Result<Tensor> {
        let (batch, seq_len, hidden) = xs.dims3()?;
        let qkv = self
            .wqkv
            .forward(xs)?
            .reshape((batch, seq_len, 3, self.num_heads, self.head_dim))?
            .permute((2, 0, 3, 1, 4))?;
        let q = qkv.i(0)?;
        let k = qkv.i(1)?;
        let v = qkv.i(2)?.contiguous()?;

        let (q, k) = self.rotary.apply(&q, &k, seq_len)?;
        let scale = (self.head_dim as f64).powf(-0.5);
        let out = attention(&q, &k, &v, scale, mask)?;
        self.wo
            .forward(&out.transpose(1, 2)?.reshape((batch, seq_len, hidden))?)
    }
}

#[derive(Debug)]
struct EncoderMlp {
    wi: Linear,
    wo: Linear,
}

impl EncoderMlp {
    fn load(vb: VarBuilder, cfg: &EncoderConfig) -> Result<Self> {
        Ok(Self {
            wi: maybe_biased_linear(
                cfg.hidden_size,
                2 * cfg.intermediate_size,
                cfg.mlp_bias,
                vb.pp("Wi"),
            )?,
            wo: maybe_biased_linear(
                cfg.intermediate_size,
                cfg.hidden_size,
                cfg.mlp_bias,
                vb.pp("Wo"),
            )?,
        })
    }
}

impl Module for EncoderMlp {
    fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        let projected = self.wi.forward(xs)?;
        let halves = projected.chunk(2, D::Minus1)?;
        // Exact GELU on the value half, gated by the second half.
        self.wo.forward(&(halves[0].gelu_erf()? * &halves[1])?)
    }
}

#[derive(Debug)]
struct EncoderLayer {
    attention_type: String,
    /// `None` at layer 0, where upstream uses `nn.Identity`.
    attn_norm: Option<LayerNorm>,
    attn: EncoderAttention,
    mlp_norm: LayerNorm,
    mlp: EncoderMlp,
}

impl EncoderLayer {
    fn load(
        vb: VarBuilder,
        cfg: &EncoderConfig,
        index: usize,
        rotary: Arc<Rotary>,
    ) -> Result<Self> {
        let attn_norm = if index == 0 {
            None
        } else {
            Some(norm(
                cfg.hidden_size,
                cfg.norm_eps,
                cfg.norm_bias,
                vb.pp("attn_norm"),
            )?)
        };
        Ok(Self {
            attention_type: cfg.layer_types()[index].clone(),
            attn_norm,
            attn: EncoderAttention::load(vb.pp("attn"), cfg, rotary)?,
            mlp_norm: norm(
                cfg.hidden_size,
                cfg.norm_eps,
                cfg.norm_bias,
                vb.pp("mlp_norm"),
            )?,
            mlp: EncoderMlp::load(vb.pp("mlp"), cfg)?,
        })
    }

    fn forward(&self, xs: &Tensor, masks: &AttentionMasks) -> Result<Tensor> {
        let normed = match &self.attn_norm {
            Some(norm) => norm.forward(xs)?,
            None => xs.clone(),
        };
        let xs = (xs
            + self
                .attn
                .forward(&normed, masks.for_kind(&self.attention_type))?)?;
        let normed = self.mlp_norm.forward(&xs)?;
        &xs + self.mlp.forward(&normed)?
    }
}

#[derive(Debug)]
pub struct ModernBert {
    embeddings: Embeddings,
    layers: Vec<EncoderLayer>,
    final_norm: LayerNorm,
    local_attention: usize,
}

impl ModernBert {
    pub fn load(
        vb: VarBuilder,
        cfg: &EncoderConfig,
        dtype: DType,
        device: &Device,
    ) -> Result<Self> {
        // Only two thetas exist, so the tables are built once and shared.
        let global = Arc::new(Rotary::new(
            cfg.head_dim(),
            cfg.max_position_embeddings,
            cfg.rope_base(FULL_ATTENTION),
            dtype,
            device,
        )?);
        let local = Arc::new(Rotary::new(
            cfg.head_dim(),
            cfg.max_position_embeddings,
            cfg.rope_base(crate::config::SLIDING_ATTENTION),
            dtype,
            device,
        )?);

        let layers_vb = vb.pp("layers");
        let mut layers = Vec::with_capacity(cfg.num_hidden_layers);
        for index in 0..cfg.num_hidden_layers {
            let rotary = if cfg.layer_types()[index] == FULL_ATTENTION {
                Arc::clone(&global)
            } else {
                Arc::clone(&local)
            };
            layers.push(EncoderLayer::load(
                layers_vb.pp(index.to_string()),
                cfg,
                index,
                rotary,
            )?);
        }

        Ok(Self {
            embeddings: Embeddings::load(vb.pp("embeddings"), cfg)?,
            layers,
            final_norm: norm(
                cfg.hidden_size,
                cfg.norm_eps,
                cfg.norm_bias,
                vb.pp("final_norm"),
            )?,
            local_attention: cfg.local_attention,
        })
    }

    pub fn forward(&self, input_ids: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
        let mut xs = self.embeddings.forward(input_ids)?;
        let masks = AttentionMasks::build(attention_mask, self.local_attention, xs.dtype())?;
        for layer in &self.layers {
            xs = layer.forward(&xs, &masks)?;
        }
        self.final_norm.forward(&xs)
    }
}

// ------------------------------------------------------------ decision head

#[derive(Debug)]
struct HeadAttention {
    in_proj: Linear,
    out_proj: Linear,
    num_heads: usize,
    head_dim: usize,
}

impl HeadAttention {
    fn load(vb: VarBuilder, dims: usize) -> Result<Self> {
        let num_heads = std::cmp::max(1, dims / 64);
        if dims % num_heads != 0 {
            candle_core::bail!("Decision head dimensions must be divisible by its head count");
        }
        Ok(Self {
            in_proj: linear(dims, 3 * dims, vb.pp("in_proj"))?,
            out_proj: linear(dims, dims, vb.pp("out_proj"))?,
            num_heads,
            head_dim: dims / num_heads,
        })
    }

    fn forward(&self, xs: &Tensor, mask: &Tensor) -> Result<Tensor> {
        let (batch, seq_len, dims) = xs.dims3()?;
        let qkv = self
            .in_proj
            .forward(xs)?
            .reshape((batch, seq_len, 3, self.num_heads, self.head_dim))?
            .permute((2, 0, 3, 1, 4))?;
        let q = qkv.i(0)?.contiguous()?;
        let k = qkv.i(1)?.contiguous()?;
        let v = qkv.i(2)?.contiguous()?;
        let scale = (self.head_dim as f64).powf(-0.5);
        let out = attention(&q, &k, &v, scale, mask)?;
        self.out_proj
            .forward(&out.transpose(1, 2)?.reshape((batch, seq_len, dims))?)
    }
}

#[derive(Debug)]
struct HeadLayer {
    self_attn: HeadAttention,
    norm1: LayerNorm,
    norm2: LayerNorm,
    linear1: Linear,
    linear2: Linear,
}

impl HeadLayer {
    fn load(vb: VarBuilder, dims: usize) -> Result<Self> {
        Ok(Self {
            self_attn: HeadAttention::load(vb.pp("self_attn"), dims)?,
            // These norms carry a bias, matching torch's TransformerEncoderLayer.
            norm1: layer_norm(dims, LayerNormConfig::default(), vb.pp("norm1"))?,
            norm2: layer_norm(dims, LayerNormConfig::default(), vb.pp("norm2"))?,
            linear1: linear(dims, 4 * dims, vb.pp("linear1"))?,
            linear2: linear(4 * dims, dims, vb.pp("linear2"))?,
        })
    }

    fn forward(&self, xs: &Tensor, mask: &Tensor) -> Result<Tensor> {
        let normed = self.norm1.forward(xs)?;
        let xs = (xs + self.self_attn.forward(&normed, mask)?)?;
        let normed = self.norm2.forward(&xs)?;
        // ReLU, not GELU: torch's TransformerEncoderLayer defaults to ReLU even
        // though the encoder and scoring head use GELU.
        let ff = self
            .linear2
            .forward(&self.linear1.forward(&normed)?.relu()?)?;
        &xs + ff
    }
}

// ---------------------------------------------------------- decision model

/// The full Laya decision model: encoder, decision head, scorer and action head.
#[derive(Debug)]
pub struct DecisionModel {
    encoder: ModernBert,
    head: Vec<HeadLayer>,
    type_emb: Embedding,
    scorer_norm: LayerNorm,
    scorer_in: Linear,
    scorer_out: Linear,
    act_in: Linear,
    act_out: Linear,
    act_dtype: DType,
}

/// One forward pass: masked scoring logits and action probabilities' logits.
#[derive(Debug, Clone)]
pub struct ModelOutput {
    /// `(batch, slots)` in f32, already masked to `-1e4` on unused slots.
    pub logits: Tensor,
    /// `(batch, act_classes)` in f32.
    pub action: Tensor,
}

impl DecisionModel {
    pub fn load(
        vb: VarBuilder,
        encoder_cfg: &EncoderConfig,
        agent_cfg: &AgentConfig,
        dtype: DType,
        device: &Device,
    ) -> Result<Self> {
        let dims = encoder_cfg.hidden_size;
        let head_vb = vb.pp("head").pp("layers");
        let mut head = Vec::with_capacity(agent_cfg.head_layers);
        for index in 0..agent_cfg.head_layers {
            head.push(HeadLayer::load(head_vb.pp(index.to_string()), dims)?);
        }

        let scorer = vb.pp("scorer").pp("layers");
        let act = vb.pp("act_head").pp("layers");

        Ok(Self {
            encoder: ModernBert::load(vb.pp("encoder"), encoder_cfg, dtype, device)?,
            head,
            type_emb: embedding(3, dims, vb.pp("type_emb"))?,
            // Sequential(LayerNorm, Linear, GELU, Linear): index 2 has no parameters.
            scorer_norm: layer_norm(dims, LayerNormConfig::default(), scorer.pp("0"))?,
            scorer_in: linear(dims, dims, scorer.pp("1"))?,
            scorer_out: linear(dims, 1, scorer.pp("3"))?,
            // Sequential(Linear, GELU, Linear).
            act_in: linear(dims + 4, 256, act.pp("0"))?,
            act_out: linear(256, agent_cfg.act_classes, act.pp("2"))?,
            act_dtype: dtype,
        })
    }

    /// Run a prepared batch.
    ///
    /// * `input_ids`, `attention_mask`: `(batch, seq)`
    /// * `marker_pos`, `marker_mask`: `(batch, slots)`
    /// * `qtype`: `(batch,)`
    pub fn forward(
        &self,
        input_ids: &Tensor,
        attention_mask: &Tensor,
        marker_pos: &Tensor,
        marker_mask: &Tensor,
        qtype: &Tensor,
    ) -> Result<ModelOutput> {
        let hidden = self.encoder.forward(input_ids, attention_mask)?;
        let (batch, seq_len, dims) = hidden.dims3()?;

        // Add the question-type embedding to every position.
        let type_bias = self.type_emb.forward(qtype)?.reshape((batch, 1, dims))?;
        let mut xs = hidden.broadcast_add(&type_bias)?;

        // The head sees key validity only; padded queries may attend freely.
        let head_mask = {
            let device = attention_mask.device();
            let valid = attention_mask
                .to_dtype(DType::U8)?
                .reshape((batch, 1, 1, seq_len))?;
            let zeros = Tensor::zeros((batch, 1, 1, seq_len), DType::F32, device)?;
            let blocked = Tensor::full(f32::NEG_INFINITY, (batch, 1, 1, seq_len), device)?;
            valid.where_cond(&zeros, &blocked)?.to_dtype(xs.dtype())?
        };
        for layer in &self.head {
            xs = layer.forward(&xs, &head_mask)?;
        }

        // Gather the hidden state at each option's marker position.
        let slots = marker_pos.dim(1)?;
        let indices = marker_pos
            .clamp(0i64, i64::MAX)?
            .to_dtype(DType::U32)?
            .reshape((batch, slots, 1))?
            .broadcast_as((batch, slots, dims))?
            .contiguous()?;
        let markers = xs.gather(&indices, 1)?;

        let scored = self.scorer_out.forward(
            &self
                .scorer_in
                .forward(&self.scorer_norm.forward(&markers)?)?
                .gelu_erf()?,
        )?;
        let logits = scored.squeeze(D::Minus1)?.to_dtype(DType::F32)?;

        // Unused slots are pushed far below any real score.
        let masked_fill = Tensor::full(MASKED_LOGIT as f32, (batch, slots), logits.device())?;
        let logits = marker_mask
            .to_dtype(DType::U8)?
            .where_cond(&logits, &masked_fill)?;

        let probabilities = softmax_last_dim(&logits)?;
        // `k` is the real option count, floored at 2 to match the padded layout.
        let counts = marker_mask
            .to_dtype(DType::F32)?
            .sum(D::Minus1)?
            .clamp(2f32, f32::MAX)?;

        let clamped = probabilities.clamp(1e-9f32, f32::MAX)?;
        let entropy = probabilities
            .mul(&clamped.log()?)?
            .sum(D::Minus1)?
            .neg()?
            .div(&counts.log()?)?;

        let (sorted, _) = probabilities.sort_last_dim(true)?;
        let top1 = sorted.i((.., slots - 1))?;
        let top2 = sorted.i((.., slots - 2))?;
        let features = Tensor::stack(
            &[top1.clone(), (&top1 - &top2)?, entropy, (counts / 255.0)?],
            D::Minus1,
        )?;

        let pooled = Tensor::cat(&[xs.i((.., 0))?.to_dtype(DType::F32)?, features], D::Minus1)?;
        let action = self
            .act_out
            .forward(
                &self
                    .act_in
                    .forward(&pooled.to_dtype(self.act_dtype)?)?
                    .gelu_erf()?,
            )?
            .to_dtype(DType::F32)?;

        Ok(ModelOutput { logits, action })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `(seq, seq)` sliding mask for an all-valid attention mask.
    fn sliding(device: &Device, seq_len: usize, window: usize) -> Vec<Vec<f32>> {
        let attention = Tensor::ones((1, seq_len), DType::U8, device).unwrap();
        let masks = AttentionMasks::build(&attention, window, DType::F32).unwrap();
        masks
            .sliding
            .reshape((seq_len, seq_len))
            .unwrap()
            .to_vec2::<f32>()
            .unwrap()
    }

    #[test]
    fn sliding_mask_allows_exactly_the_local_window() {
        let (seq_len, window) = (8usize, 6usize);
        let half = (window / 2) as i64;
        let rows = sliding(&Device::Cpu, seq_len, window);

        for (i, row) in rows.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                let inside = (i as i64 - j as i64).abs() <= half;
                if inside {
                    assert_eq!(*value, 0.0, "({i}, {j}) is inside the window");
                } else {
                    assert!(value.is_infinite() && *value < 0.0, "({i}, {j}) is outside");
                }
            }
        }
    }

    /// Regression test for the `uabs I64 not implemented` Metal failure: the
    /// mask must build on the GPU and agree with the CPU bit for bit.
    #[cfg(feature = "metal")]
    #[test]
    fn sliding_mask_builds_on_metal_and_matches_cpu() {
        let Ok(metal) = Device::new_metal(0) else {
            return;
        };
        let (seq_len, window) = (8usize, 6usize);
        assert_eq!(
            sliding(&metal, seq_len, window),
            sliding(&Device::Cpu, seq_len, window)
        );
    }
}
