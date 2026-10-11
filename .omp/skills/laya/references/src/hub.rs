//! Checkpoint resolution against the Hugging Face Hub.

use hf_hub::{split_id, HFClientBuilder};
use std::path::{Path, PathBuf};

use crate::agent::{Agent, AgentBuilder};
use crate::router::{ModelName, Router};
use crate::{Error, Result};

/// Only these files are fetched, matching upstream's `allow_patterns`.
const CHECKPOINT_PATTERNS: [&str; 5] = [
    "model.safetensors",
    "rl_agent_config.json",
    "encoder/config.json",
    "tokenizer/*",
    "mlx_config.json",
];

/// Download a checkpoint and return the directory holding it.
///
/// A path that exists on disk is returned unchanged, matching upstream's
/// `resolve_model`.
pub fn resolve_model(
    model_id_or_path: &str,
    subfolder: Option<&str>,
    revision: Option<&str>,
    token: Option<String>,
) -> Result<PathBuf> {
    if let Some(subfolder) = subfolder {
        let path = Path::new(subfolder);
        if path.is_absolute() || path.components().any(|c| c.as_os_str() == "..") {
            return Err(Error::Config(
                "subfolder must be a relative path inside the model repository".into(),
            ));
        }
    }

    let local = PathBuf::from(expand_home(model_id_or_path));
    if local.exists() {
        return Ok(match subfolder {
            Some(sub) => local.join(sub),
            None => local,
        });
    }
    if model_id_or_path.starts_with('/')
        || model_id_or_path.starts_with("./")
        || model_id_or_path.starts_with("../")
        || model_id_or_path.starts_with('~')
    {
        return Err(Error::Config(format!(
            "Local model directory does not exist: {model_id_or_path}"
        )));
    }

    let mut builder = HFClientBuilder::new();
    if let Some(token) = token.or_else(|| std::env::var("HF_TOKEN").ok()) {
        builder = builder.token(token);
    }
    let client = builder
        .build_sync()
        .map_err(|error| Error::Hub(error.to_string()))?;

    let (owner, name) = split_id(model_id_or_path);
    let repository = client.model(owner, name);

    // Restrict to the requested subfolder so the bundle repository does not
    // pull all three checkpoints when only one is wanted.
    let patterns: Vec<String> = CHECKPOINT_PATTERNS
        .iter()
        .map(|pattern| match subfolder {
            Some(sub) => format!("{}/{pattern}", sub.trim_end_matches('/')),
            None => (*pattern).to_string(),
        })
        .collect();

    let snapshot = repository
        .snapshot_download()
        .maybe_revision(revision.map(str::to_string))
        .allow_patterns(patterns)
        .send()
        .map_err(|error| Error::Hub(format!("{model_id_or_path}: {error}")))?;

    Ok(match subfolder {
        Some(sub) => snapshot.join(sub),
        None => snapshot,
    })
}

fn expand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return Path::new(&home).join(rest).to_string_lossy().into_owned();
        }
    }
    path.to_string()
}

impl Agent {
    /// Load a checkpoint by Hub id or local path, with default settings.
    pub fn from_pretrained(model_id_or_path: &str) -> Result<Self> {
        Self::from_pretrained_with(model_id_or_path, None, None, AgentBuilder::new())
    }

    /// Load a checkpoint, choosing a subfolder, revision and options.
    pub fn from_pretrained_with(
        model_id_or_path: &str,
        subfolder: Option<&str>,
        revision: Option<&str>,
        options: AgentBuilder,
    ) -> Result<Self> {
        let dir = resolve_model(model_id_or_path, subfolder, revision, None)?;
        options.build(dir)
    }
}

impl Router {
    /// Download and build a checkpoint, then keep it resident.
    pub fn load(
        &mut self,
        model: ModelName,
        options: AgentBuilder,
    ) -> Result<std::sync::Arc<Agent>> {
        if let Some(agent) = self.get(model) {
            return Ok(agent);
        }
        let (repo, subfolder) = model.bundle_location();
        let dir = resolve_model(repo, subfolder, None, None)?;
        let agent = std::sync::Arc::new(options.build(dir)?);
        self.attach(model, std::sync::Arc::clone(&agent));
        Ok(agent)
    }

    /// Download and build several checkpoints up front.
    ///
    /// A cold load costs seconds while detection costs microseconds, so a
    /// server that alternates languages should preload rather than rely on the
    /// LRU, which would otherwise reload on nearly every request.
    pub fn preload(
        &mut self,
        models: &[ModelName],
        options: impl Fn() -> AgentBuilder,
    ) -> Result<()> {
        self.set_max_loaded_at_least(models.len());
        for model in models {
            self.load(*model, options())?;
        }
        Ok(())
    }
}
