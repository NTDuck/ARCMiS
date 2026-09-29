//! Append-only ledgers: decisions, failures, observations. One JSONL line
//! per event; meta-harness reads these after the run.

use std::io::Write as _;
use std::path::PathBuf;

use anyhow::Context as _;
use serde::Deserialize;
use serde::Serialize;

/// One orchestrator decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// RFC 3339 UTC timestamp.
    pub at: String,
    /// Phase the decision belongs to.
    pub phase: String,
    /// What was decided (e.g. `delegate`, `replan`, `escalate`, `done`).
    pub action: String,
    /// Machine-readable detail (role, batch id, model, ...).
    #[serde(default)]
    pub detail: serde_json::Value,
    /// Reasoning text (STE-clean).
    #[serde(default)]
    pub reasoning: String,
}

/// One failure event with its harness-side classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Failure {
    /// RFC 3339 UTC timestamp.
    pub at: String,
    /// Phase the failure occurred in.
    pub phase: String,
    /// Failure category (mirrors the failure-analyst taxonomy).
    pub category: String,
    /// Root cause text.
    pub root_cause: String,
    /// Suggested action text.
    #[serde(default)]
    pub suggested_action: String,
}

/// One passive observation (progress, stagnation, token budget).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// RFC 3339 UTC timestamp.
    pub at: String,
    /// Observation kind (e.g. `token_budget`, `stagnation`, `phase_duration`).
    pub kind: String,
    /// Machine-readable payload.
    #[serde(default)]
    pub detail: serde_json::Value,
}

/// Append-only ledger rooted at `dir`.
#[derive(Debug, Clone)]
pub struct Ledger {
    dir: PathBuf,
}

impl Ledger {
    /// Bind a ledger to `dir`.
    #[must_use]
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
        }
    }

    /// Append one decision to `decisions.jsonl`.
    pub fn append_decision(&self, decision: &Decision) -> anyhow::Result<()> {
        self.append("decisions.jsonl", decision)
    }

    /// Append one failure to `failures.jsonl`.
    pub fn append_failure(&self, failure: &Failure) -> anyhow::Result<()> {
        self.append("failures.jsonl", failure)
    }

    /// Append one observation to `observations.jsonl`.
    pub fn append_observation(&self, observation: &Observation) -> anyhow::Result<()> {
        self.append("observations.jsonl", observation)
    }

    /// Read every decision in order.
    pub fn read_decisions(&self) -> anyhow::Result<Vec<Decision>> {
        self.read_all("decisions.jsonl")
    }

    /// Read every failure in order.
    pub fn read_failures(&self) -> anyhow::Result<Vec<Failure>> {
        self.read_all("failures.jsonl")
    }

    /// Read every observation in order.
    pub fn read_observations(&self) -> anyhow::Result<Vec<Observation>> {
        self.read_all("observations.jsonl")
    }

    /// Append one serializable line atomically: open, seek-end, write, flush.
    fn append<T: Serialize>(&self, file: &str, value: &T) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(file);
        let mut handle = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("open {}", path.display()))?;
        let mut line = serde_json::to_string(value)?;
        line.push('\n');
        handle.write_all(line.as_bytes())?;
        handle.flush()?;
        Ok(())
    }

    /// Read all lines of one ledger file in append order.
    fn read_all<T: for<'de> Deserialize<'de>>(&self, file: &str) -> anyhow::Result<Vec<T>> {
        let path = self.dir.join(file);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut values = Vec::new();
        for (index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            values.push(serde_json::from_str(line).with_context(|| format!("{} line {}", path.display(), index + 1))?);
        }
        Ok(values)
    }
}
