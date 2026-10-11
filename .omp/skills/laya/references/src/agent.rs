//! The public inference runtime. Prompt and result formats follow upstream Laya.

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use laya_core::calibrate::{calibrated_softmax, confidence_from_probs};
use laya_core::prompt::{build_prefix, extend_with_state, serialize_state, Prefix, Sequence};
use laya_core::pycompat::round4;
use laya_core::question::{Question, QuestionKind};
use laya_core::Tokenizer;
use rayon::prelude::*;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::config::{AgentConfig, EncoderConfig};
use crate::model::DecisionModel;
use crate::weights::SanitizedBackend;
use crate::{Error, Result};

/// Files every checkpoint must contain.
const REQUIRED_FILES: [&str; 3] = [
    "model.safetensors",
    "rl_agent_config.json",
    "encoder/config.json",
];

/// Questions per forward pass, matching upstream's default.
const DEFAULT_BATCH_SIZE: usize = 16;
/// Upstream bounds its prefix cache at this many questions.
const PREFIX_CACHE_CAPACITY: usize = 128;
/// Below this many questions, threading costs more than it saves.
const PARALLEL_THRESHOLD: usize = 4;

/// Parse a dtype name the way the Python API spells it.
pub fn parse_dtype(name: &str) -> Result<DType> {
    match name {
        "float32" => Ok(DType::F32),
        "float16" => Ok(DType::F16),
        "bfloat16" => Ok(DType::BF16),
        other => Err(Error::Config(format!(
            "dtype must be one of [\"float32\", \"float16\", \"bfloat16\"], got {other:?}"
        ))),
    }
}

/// One prepared question, ready for collation.
#[derive(Clone, Debug)]
struct Item {
    ids: Vec<u32>,
    markers: Vec<usize>,
    qtype: QuestionKind,
    truncated: bool,
}

/// A collated batch of prepared questions.
#[derive(Debug)]
struct Batch {
    input_ids: Tensor,
    attention_mask: Tensor,
    marker_pos: Tensor,
    marker_mask: Tensor,
    qtype: Tensor,
}

/// One answered question.
///
/// Values are stored already rounded to four decimals, matching what the
/// upstream API returns.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    pub kind: QuestionKind,
    pub confidence: f64,
    pub act_probability: f64,
    /// Option labels in index order: choice labels, or `"0"`, `"1"`, … for a score.
    pub labels: Vec<String>,
    /// Probability per option, aligned with `labels`.
    pub probabilities: Vec<f64>,
    /// Selected label, for `choice`.
    pub choice: Option<String>,
    /// Expected zero-based rubric level, for `score`.
    pub score: Option<f64>,
    /// Rubric legend, for `score`.
    pub legend: Option<Map<String, Value>>,
    /// P(true), for `noul`.
    pub noul: Option<f64>,
}

impl Answer {
    /// Render in upstream's exact key order.
    pub fn to_value(&self) -> Value {
        let mut out = Map::new();
        out.insert("type".into(), Value::String(self.kind.name().into()));
        out.insert("confidence".into(), number(self.confidence));
        let mut action = Map::new();
        action.insert("act_probability".into(), number(self.act_probability));
        out.insert("action".into(), Value::Object(action));

        match self.kind {
            QuestionKind::Choice => {
                out.insert(
                    "choice".into(),
                    Value::String(self.choice.clone().unwrap_or_default()),
                );
                out.insert("probabilities".into(), self.probability_map());
            }
            QuestionKind::Score => {
                out.insert("score".into(), number(self.score.unwrap_or(0.0)));
                out.insert(
                    "legend".into(),
                    Value::Object(self.legend.clone().unwrap_or_default()),
                );
                out.insert("probabilities".into(), self.probability_map());
            }
            QuestionKind::Noul => {
                out.insert("noul".into(), number(self.noul.unwrap_or(0.0)));
            }
        }
        Value::Object(out)
    }

    fn probability_map(&self) -> Value {
        let mut map = Map::new();
        for (label, probability) in self.labels.iter().zip(&self.probabilities) {
            map.insert(label.clone(), number(*probability));
        }
        Value::Object(map)
    }
}

/// Serialize a rounded float, preserving CPython's `1.0` rather than `1`.
fn number(value: f64) -> Value {
    serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)
}

/// The full result of one `predict` call.
#[derive(Clone, Debug)]
pub struct Prediction {
    pub model: String,
    /// Answers in question order.
    pub answers: Map<String, Value>,
    /// Typed answers, same order and keys as `answers`.
    pub typed: Vec<(String, Answer)>,
    /// Total across every sequence in the call, as upstream reports it.
    pub input_tokens: usize,
    pub output_tokens: usize,
    /// Token count of each question's sequence, in question order.
    ///
    /// `input_tokens` is a batch total, so it cannot distinguish several short
    /// sequences from one that was cut at `max_len`; these can.
    pub sequence_lengths: Vec<usize>,
    /// True when any sequence lost part of its state to `max_len`.
    pub truncated: bool,
    /// Present only on results produced through a [`crate::Router`].
    pub routing: Option<Value>,
}

impl Prediction {
    /// Render in upstream's exact shape.
    pub fn to_value(&self) -> Value {
        let mut out = Map::new();
        out.insert("model".into(), Value::String(self.model.clone()));
        out.insert("answers".into(), Value::Object(self.answers.clone()));
        let mut usage = Map::new();
        usage.insert("input_tokens".into(), Value::from(self.input_tokens));
        usage.insert("output_tokens".into(), Value::from(self.output_tokens));
        out.insert("usage".into(), Value::Object(usage));
        if let Some(routing) = &self.routing {
            out.insert("routing".into(), routing.clone());
        }
        Value::Object(out)
    }

    /// Upstream's shape plus the per-sequence usage detail.
    ///
    /// Kept separate from [`Self::to_value`] so the default rendering stays
    /// byte-compatible with upstream, which emits only the two token counts.
    pub fn to_value_detailed(&self) -> Value {
        let mut out = self.to_value();
        if let Some(usage) = out.get_mut("usage").and_then(|usage| usage.as_object_mut()) {
            usage.insert(
                "sequence_lengths".into(),
                Value::from(self.sequence_lengths.clone()),
            );
            usage.insert("truncated".into(), Value::Bool(self.truncated));
        }
        out
    }

    /// Look up one answer by question id.
    pub fn answer(&self, question_id: &str) -> Option<&Answer> {
        self.typed
            .iter()
            .find(|(id, _)| id == question_id)
            .map(|(_, answer)| answer)
    }
}

/// Bounded tokenized-prefix reuse.
///
/// Only the tokenized *question* prefix is cached; encoder states and
/// predictions never are, so every question still gets its own forward pass.
/// The key is the question's content rather than upstream's tokenizer identity,
/// which makes the cache safe to share and to persist.
#[derive(Debug, Default)]
struct PrefixCache {
    entries: HashMap<String, Prefix>,
    order: Vec<String>,
}

impl PrefixCache {
    fn key(question: &Question, head_max_len: usize) -> String {
        let mut key = String::with_capacity(64);
        key.push_str(question.kind.name());
        key.push('\u{1}');
        key.push_str(&head_max_len.to_string());
        key.push('\u{1}');
        key.push_str(&question.instructions);
        for option in question.render_options() {
            key.push('\u{1}');
            key.push_str(&option);
        }
        key
    }

    fn get_or_insert(
        &mut self,
        question: &Question,
        head_max_len: usize,
        build: impl FnOnce() -> Result<Prefix>,
    ) -> Result<Prefix> {
        let key = Self::key(question, head_max_len);
        if let Some(prefix) = self.entries.get(&key) {
            let prefix = prefix.clone();
            self.touch(&key);
            return Ok(prefix);
        }
        let prefix = build()?;
        self.entries.insert(key.clone(), prefix.clone());
        self.order.push(key);
        while self.order.len() > PREFIX_CACHE_CAPACITY {
            let evicted = self.order.remove(0);
            self.entries.remove(&evicted);
        }
        Ok(prefix)
    }

    fn touch(&mut self, key: &str) {
        if let Some(index) = self.order.iter().position(|entry| entry == key) {
            let key = self.order.remove(index);
            self.order.push(key);
        }
    }
}

/// Builder for [`Agent`].
pub struct AgentBuilder {
    dtype: DType,
    device: Device,
    batch_size: usize,
    pad_to_multiple: Option<usize>,
    cache_prompts: bool,
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self {
            dtype: DType::F16,
            device: Device::Cpu,
            batch_size: DEFAULT_BATCH_SIZE,
            pad_to_multiple: None,
            cache_prompts: false,
        }
    }
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// `"float32"`, `"float16"` (the default) or `"bfloat16"`.
    pub fn dtype(mut self, dtype: DType) -> Self {
        self.dtype = dtype;
        self
    }

    pub fn device(mut self, device: Device) -> Self {
        self.device = device;
        self
    }

    /// Questions per forward pass. Defaults to 16, as upstream does.
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size.max(1);
        self
    }

    /// Pad sequence length up to a multiple, to reduce shape churn.
    pub fn pad_to_multiple(mut self, pad_to_multiple: Option<usize>) -> Self {
        self.pad_to_multiple = pad_to_multiple.filter(|value| *value >= 1);
        self
    }

    /// Reuse tokenized question prefixes across calls.
    pub fn cache_prompts(mut self, cache_prompts: bool) -> Self {
        self.cache_prompts = cache_prompts;
        self
    }

    /// Load a checkpoint from a local directory.
    pub fn build(self, model_dir: impl AsRef<Path>) -> Result<Agent> {
        Agent::load(model_dir.as_ref(), self)
    }
}

/// A loaded Laya checkpoint.
pub struct Agent {
    model_dir: PathBuf,
    encoder_cfg: EncoderConfig,
    agent_cfg: AgentConfig,
    tokenizer: Tokenizer,
    model: DecisionModel,
    device: Device,
    dtype: DType,
    batch_size: usize,
    pad_to_multiple: Option<usize>,
    prefix_cache: Option<Mutex<PrefixCache>>,
}

impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent")
            .field("model_dir", &self.model_dir)
            .field("dtype", &self.dtype)
            .field("batch_size", &self.batch_size)
            .finish_non_exhaustive()
    }
}

impl Agent {
    /// Configure a load.
    pub fn builder() -> AgentBuilder {
        AgentBuilder::new()
    }

    /// Load a checkpoint from a local directory with default settings.
    pub fn from_dir(model_dir: impl AsRef<Path>) -> Result<Self> {
        AgentBuilder::new().build(model_dir)
    }

    fn load(model_dir: &Path, options: AgentBuilder) -> Result<Self> {
        for name in REQUIRED_FILES {
            let path = model_dir.join(name);
            if !path.is_file() {
                return Err(Error::IncompleteCheckpoint(path));
            }
        }

        let agent_raw: Value = serde_json::from_str(&std::fs::read_to_string(
            model_dir.join("rl_agent_config.json"),
        )?)?;
        let encoder_raw: Value = serde_json::from_str(&std::fs::read_to_string(
            model_dir.join("encoder/config.json"),
        )?)?;

        let encoder_cfg = EncoderConfig::from_value(&encoder_raw)?;
        let agent_cfg = AgentConfig::from_value(&agent_raw, encoder_cfg.max_position_embeddings)?;
        let tokenizer = Tokenizer::from_dir(model_dir.join("tokenizer"))?;

        // Memory-map the weights and expose them under sanitized names, so no
        // copy of the checkpoint is made just to rename parameters.
        let weights_path = model_dir.join("model.safetensors");
        let mmaped = unsafe {
            candle_core::safetensors::MmapedSafetensors::new(&weights_path)
                .map_err(Error::Candle)?
        };
        let stored_names: Vec<String> =
            mmaped.tensors().into_iter().map(|(name, _)| name).collect();
        let backend = SanitizedBackend::new(Box::new(mmaped), stored_names);
        let vb = VarBuilder::from_backend(Box::new(backend), options.dtype, options.device.clone());

        let model =
            DecisionModel::load(vb, &encoder_cfg, &agent_cfg, options.dtype, &options.device)?;

        Ok(Self {
            model_dir: model_dir.to_path_buf(),
            encoder_cfg,
            agent_cfg,
            tokenizer,
            model,
            device: options.device,
            dtype: options.dtype,
            batch_size: options.batch_size,
            pad_to_multiple: options.pad_to_multiple,
            prefix_cache: options
                .cache_prompts
                .then(|| Mutex::new(PrefixCache::default())),
        })
    }

    pub fn model_dir(&self) -> &Path {
        &self.model_dir
    }

    pub fn config(&self) -> &AgentConfig {
        &self.agent_cfg
    }

    pub fn encoder_config(&self) -> &EncoderConfig {
        &self.encoder_cfg
    }

    pub fn tokenizer(&self) -> &Tokenizer {
        &self.tokenizer
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn dtype(&self) -> DType {
        self.dtype
    }

    /// Validate and tokenize every question against a state.
    fn prepare(
        &self,
        state: &Value,
        questions: &Map<String, Value>,
    ) -> Result<(Vec<Item>, Vec<Question>)> {
        if questions.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let max_len = self.agent_cfg.max_len;
        let head_max_len = self.agent_cfg.head_max_len;

        // The state is tokenized once and shared by every question, which is
        // where the prefix cache earns its keep.
        let state_text = serialize_state(state).replace(self.tokenizer.mask_token.as_str(), " ");
        let state_ids = self.tokenizer.encode(&state_text)?;

        let entries: Vec<(&String, &Value)> = questions.iter().collect();
        let build_one = |(qid, definition): &(&String, &Value)| -> Result<(Item, Question)> {
            let question = Question::from_value(definition)?;
            let option_count = question.render_options().len();

            let prefix = match &self.prefix_cache {
                Some(cache) => {
                    let mut cache = cache.lock().expect("prefix cache is not poisoned");
                    cache.get_or_insert(&question, head_max_len, || {
                        build_prefix(&self.tokenizer, &question, head_max_len).map_err(Error::Core)
                    })?
                }
                None => build_prefix(&self.tokenizer, &question, head_max_len)?,
            };

            let Sequence {
                ids,
                markers,
                truncated,
            } = extend_with_state(&self.tokenizer, &prefix, &state_ids, max_len, false);
            if markers.len() != option_count {
                return Err(Error::Core(laya_core::Error::Question(format!(
                    "Question {qid:?} has too many options for the token budget"
                ))));
            }
            let qtype = question.kind;
            Ok((
                Item {
                    ids,
                    markers,
                    qtype,
                    truncated,
                },
                question,
            ))
        };

        // Tokenization is independent per question and dominates preparation
        // time for large requests, so fan it out once it is worth the overhead.
        let prepared: Vec<(Item, Question)> =
            if entries.len() >= PARALLEL_THRESHOLD && self.prefix_cache.is_none() {
                entries.par_iter().map(build_one).collect::<Result<_>>()?
            } else {
                entries.iter().map(build_one).collect::<Result<_>>()?
            };

        Ok(prepared.into_iter().unzip())
    }

    /// Pad a chunk of prepared questions into dense tensors.
    fn collate(&self, items: &[Item]) -> Result<Batch> {
        let rows = items.len();
        let mut length = items.iter().map(|item| item.ids.len()).max().unwrap_or(0);
        if let Some(multiple) = self.pad_to_multiple {
            length = length.div_ceil(multiple) * multiple;
            length = std::cmp::min(length, self.agent_cfg.max_len);
        }
        // At least two marker slots, so the top-2 features are always defined.
        let slots = std::cmp::max(
            2,
            items
                .iter()
                .map(|item| item.markers.len())
                .max()
                .unwrap_or(0),
        );

        let pad_id = self.tokenizer.pad_token_id;
        let mut input_ids = vec![pad_id; rows * length];
        let mut attention_mask = vec![0u8; rows * length];
        let mut marker_pos = vec![0i64; rows * slots];
        let mut marker_mask = vec![0u8; rows * slots];
        let mut qtype = Vec::with_capacity(rows);

        for (row, item) in items.iter().enumerate() {
            let used = std::cmp::min(item.ids.len(), length);
            input_ids[row * length..row * length + used].copy_from_slice(&item.ids[..used]);
            attention_mask[row * length..row * length + used].fill(1);
            for (slot, marker) in item.markers.iter().enumerate() {
                marker_pos[row * slots + slot] = *marker as i64;
                marker_mask[row * slots + slot] = 1;
            }
            qtype.push(item.qtype.index() as u32);
        }

        Ok(Batch {
            input_ids: Tensor::from_vec(input_ids, (rows, length), &self.device)?,
            attention_mask: Tensor::from_vec(attention_mask, (rows, length), &self.device)?,
            marker_pos: Tensor::from_vec(marker_pos, (rows, slots), &self.device)?,
            marker_mask: Tensor::from_vec(marker_mask, (rows, slots), &self.device)?,
            qtype: Tensor::from_vec(qtype, rows, &self.device)?,
        })
    }

    /// Answer every question about a state.
    pub fn predict(&self, state: &Value, questions: &Value) -> Result<Prediction> {
        let questions = questions.as_object().ok_or_else(|| {
            Error::Core(laya_core::Error::Question(
                "questions must be a dictionary keyed by question id".into(),
            ))
        })?;
        self.predict_map(state, questions)
    }

    /// Alias for [`Agent::predict`], matching upstream's `system_one`.
    pub fn system_one(&self, state: &Value, questions: &Value) -> Result<Prediction> {
        self.predict(state, questions)
    }

    /// Answer every question, taking the question map directly.
    pub fn predict_map(&self, state: &Value, questions: &Map<String, Value>) -> Result<Prediction> {
        let (items, internal) = self.prepare(state, questions)?;
        let question_ids: Vec<&String> = questions.keys().collect();

        let mut answers = Map::new();
        let mut typed = Vec::with_capacity(items.len());

        for (chunk_index, chunk) in items.chunks(self.batch_size).enumerate() {
            let batch = self.collate(chunk)?;
            let output = self.model.forward(
                &batch.input_ids,
                &batch.attention_mask,
                &batch.marker_pos,
                &batch.marker_mask,
                &batch.qtype,
            )?;

            let logits: Vec<f32> = output.logits.flatten_all()?.to_vec1()?;
            let action: Vec<f32> = output.action.flatten_all()?.to_vec1()?;
            if logits.iter().chain(action.iter()).any(|v| !v.is_finite()) {
                return Err(Error::NonFinite);
            }

            let slots = output.logits.dim(1)?;
            let act_classes = output.action.dim(1)?;
            let offset = chunk_index * self.batch_size;

            for (row, item) in chunk.iter().enumerate() {
                let index = offset + row;
                let qid = question_ids[index];
                let question = &internal[index];
                let k = item.markers.len();

                let temperature = self.agent_cfg.temperature_for(item.qtype, k);
                let row_logits = &logits[row * slots..row * slots + k];
                let probabilities = calibrated_softmax(row_logits, temperature);

                // Upstream softmaxes the action head in NumPy, after the model.
                let act_row = &action[row * act_classes..(row + 1) * act_classes];
                let act_probabilities = laya_core::calibrate::softmax(act_row);
                let act_probability = round4(act_probabilities.first().copied().unwrap_or(0.0));

                let answer = build_answer(question, &probabilities, k, act_probability);
                answers.insert(qid.to_string(), answer.to_value());
                typed.push((qid.to_string(), answer));
            }
        }

        Ok(Prediction {
            model: "laya-rl-agent".into(),
            answers,
            typed,
            input_tokens: items.iter().map(|item| item.ids.len()).sum(),
            output_tokens: 0,
            sequence_lengths: items.iter().map(|item| item.ids.len()).collect(),
            truncated: items.iter().any(|item| item.truncated),
            routing: None,
        })
    }
}

/// Assemble one answer from calibrated probabilities.
fn build_answer(
    question: &Question,
    probabilities: &[f64],
    k: usize,
    act_probability: f64,
) -> Answer {
    let rounded: Vec<f64> = probabilities.iter().copied().map(round4).collect();
    let confidence = round4(confidence_from_probs(probabilities, k));

    match question.kind {
        QuestionKind::Choice => {
            let labels = question.choice_labels().unwrap_or_default();
            let best = probabilities
                .iter()
                .enumerate()
                .fold((0usize, f64::NEG_INFINITY), |acc, (index, value)| {
                    // `argmax` keeps the first maximum, as NumPy does.
                    if *value > acc.1 {
                        (index, *value)
                    } else {
                        acc
                    }
                })
                .0;
            Answer {
                kind: QuestionKind::Choice,
                confidence,
                act_probability,
                choice: labels.get(best).cloned(),
                labels,
                probabilities: rounded,
                score: None,
                legend: None,
                noul: None,
            }
        }
        QuestionKind::Score => {
            let expected: f64 = probabilities
                .iter()
                .enumerate()
                .map(|(index, value)| index as f64 * value)
                .sum();
            Answer {
                kind: QuestionKind::Score,
                confidence,
                act_probability,
                labels: (0..probabilities.len()).map(|i| i.to_string()).collect(),
                probabilities: rounded,
                choice: None,
                score: Some(round4(expected)),
                legend: question.score_legend(),
                noul: None,
            }
        }
        QuestionKind::Noul => {
            let p_true = probabilities.get(1).copied().unwrap_or(0.0);
            Answer {
                kind: QuestionKind::Noul,
                // Upstream overwrites the entropy confidence for noul.
                confidence: round4(p_true.max(1.0 - p_true)),
                act_probability,
                labels: vec!["false".into(), "true".into()],
                probabilities: rounded,
                choice: None,
                score: None,
                legend: None,
                noul: Some(round4(p_true)),
            }
        }
    }
}
