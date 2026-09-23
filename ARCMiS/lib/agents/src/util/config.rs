//! Typed view of `assets/configs/<run>/config.yml`. Every task-specific
//! value (model, paths, languages, toolchain) lives in the yml, not in the
//! agent. See .omp/rules/config.md.

use anyhow::Context as _;
use serde::Deserialize;
use std::fs::read_to_string;
use std::path::Path;
use std::path::PathBuf;

/// Top-level config file shape.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub run: Run,
    pub output: Output,
    pub source: Source,
    /// Ledger method budgets. Ignored by the other methods.
    #[serde(default)]
    pub ledger: LedgerBudgets,
    /// Recode fix-round ceiling. Ignored by the other methods.
    #[serde(default)]
    pub recode: RecodeBudgets,
}

/// Run section: model identity and agent budget.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Run {
    pub model: String,
    pub max_turns: usize,
    pub num_ctx: u64,
    pub max_output_tokens: u64,
    pub max_retries: u32,
    /// Enable model thinking mode. Thinking improves long-horizon tool
    /// use on the supported models; disable only for a specific reason.
    pub think: bool,
    /// Sampling temperature for the model.
    pub temperature: f64,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            model: String::new(),
            max_turns: 14,
            num_ctx: 16384,
            max_output_tokens: 8192,
            max_retries: 1,
            think: true,
            temperature: 0.2,
        }
    }
}

/// Recode budgets: how many fix rounds the pipeline may run after the
/// first execution round. The paper's Algorithm 1 loops until the
/// validator passes or the ceiling hits.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct RecodeBudgets {
    pub max_rounds: usize,
}

impl Default for RecodeBudgets {
    fn default() -> Self {
        Self {
            max_rounds: 10,
        }
    }
}

/// Output section: where the transformed codebase and logs land.
#[derive(Debug, Deserialize)]
pub struct Output {
    pub dir: PathBuf,
}

/// Ledger method budgets: the manager and each fresh worker delegation
/// carry separate turn budgets (paper: fresh worker per cycle with its
/// own call allowance). `retries` covers worker delegations.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LedgerBudgets {
    pub manager_turns: usize,
    pub worker_turns: usize,
    pub retries: u32,
}

impl Default for LedgerBudgets {
    fn default() -> Self {
        Self {
            manager_turns: 30,
            worker_turns: 60,
            retries: 1,
        }
    }
}

/// Source section: input codebase, target language, and toolchain.
#[derive(Debug, Deserialize)]
pub struct Source {
    pub language: String,
    pub root: PathBuf,
    pub target: Target,
}

/// Target section: output language and test invocation.
#[derive(Debug, Deserialize)]
pub struct Target {
    pub language: String,
    pub test_command: String,
}

impl Config {
    /// Load and parse the config file at `path`.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = read_to_string(path).with_context(|| format!("config load failed for {}", path.display()))?;
        serde_yaml::from_str(&raw).with_context(|| format!("config parse failed for {}", path.display()))
    }
}
