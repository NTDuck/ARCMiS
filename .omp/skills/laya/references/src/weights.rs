//! Parameter-name mapping.
//!
//! Two checkpoint layouts have to load: the original PyTorch export
//! (`convaiinnovations/laya`) and the pre-converted MLX export
//! (`aac6fef/laya-mlx`). Upstream reconciles them in `sanitize_weights`; the
//! same mapping is applied here, but as a *view* over the memory-mapped file
//! rather than by materialising a renamed copy of every tensor.

use candle_core::{DType, Device, Shape, Tensor};
use candle_nn::var_builder::SimpleBackend;
use candle_nn::Init;

/// Map an upstream parameter name onto the name this port uses.
///
/// * PyTorch packs multi-head attention projections into `in_proj_weight` /
///   `in_proj_bias`; here they are a normal `Linear` called `in_proj`.
/// * `scorer` and `act_head` are `Sequential`s, whose children live under
///   `layers.` — already true in MLX exports, not in PyTorch ones.
pub fn sanitize_name(name: &str) -> String {
    let mut name = name
        .replace(".in_proj_weight", ".in_proj.weight")
        .replace(".in_proj_bias", ".in_proj.bias");
    for prefix in ["scorer", "act_head"] {
        let dotted = format!("{prefix}.");
        let nested = format!("{prefix}.layers.");
        if name.starts_with(&dotted) && !name.starts_with(&nested) {
            name = format!("{nested}{}", &name[dotted.len()..]);
        }
    }
    name
}

/// A `SimpleBackend` that renames on lookup.
///
/// Rather than sanitizing every key up front, the inverse mapping is computed
/// once at load time and consulted per parameter, so the underlying mmap is
/// never copied.
pub struct SanitizedBackend {
    inner: Box<dyn SimpleBackend>,
    /// Sanitized name -> name as stored in the file, only where they differ.
    aliases: std::collections::HashMap<String, String>,
}

impl SanitizedBackend {
    pub fn new(
        inner: Box<dyn SimpleBackend>,
        stored_names: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut aliases = std::collections::HashMap::new();
        for stored in stored_names {
            let sanitized = sanitize_name(&stored);
            if sanitized != stored {
                aliases.insert(sanitized, stored);
            }
        }
        Self { inner, aliases }
    }

    fn resolve<'a>(&'a self, name: &'a str) -> &'a str {
        self.aliases.get(name).map_or(name, String::as_str)
    }
}

impl SimpleBackend for SanitizedBackend {
    fn get(
        &self,
        shape: Shape,
        name: &str,
        init: Init,
        dtype: DType,
        device: &Device,
    ) -> candle_core::Result<Tensor> {
        self.inner
            .get(shape, self.resolve(name), init, dtype, device)
    }

    fn get_unchecked(
        &self,
        name: &str,
        dtype: DType,
        device: &Device,
    ) -> candle_core::Result<Tensor> {
        self.inner.get_unchecked(self.resolve(name), dtype, device)
    }

    fn contains_tensor(&self, name: &str) -> bool {
        self.inner.contains_tensor(self.resolve(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_packed_attention_projections() {
        assert_eq!(
            sanitize_name("head.layers.0.self_attn.in_proj_weight"),
            "head.layers.0.self_attn.in_proj.weight"
        );
        assert_eq!(
            sanitize_name("head.layers.0.self_attn.in_proj_bias"),
            "head.layers.0.self_attn.in_proj.bias"
        );
    }

    #[test]
    fn nests_sequential_children() {
        assert_eq!(sanitize_name("scorer.0.weight"), "scorer.layers.0.weight");
        assert_eq!(sanitize_name("act_head.2.bias"), "act_head.layers.2.bias");
    }

    #[test]
    fn already_converted_names_are_untouched() {
        for name in [
            "scorer.layers.1.weight",
            "act_head.layers.0.bias",
            "encoder.layers.3.attn.Wqkv.weight",
            "type_emb.weight",
            "temperature",
        ] {
            assert_eq!(sanitize_name(name), name);
        }
    }
}
