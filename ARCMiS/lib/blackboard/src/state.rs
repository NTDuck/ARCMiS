//! Current run state, materialized as `state.json`. The orchestrator reads it
//! on every round; the orchestrator writes it on every phase transition.

use std::path::Path;

use anyhow::Context as _;
use serde::Deserialize;
use serde::Serialize;

/// Methodology phases. Reversible transitions on failure are the contract:
/// the orchestrator may regress (e.g. MIGRATION → DISCOVERY on a missing
/// dependency) and the state file records the regression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Phase {
    /// Deterministic checks before any model call.
    Preflight,
    /// Source exploration: analyst + architect.
    Discovery,
    /// Synthesis document: `analysis/brief.md`.
    Contract,
    /// Dependency batches: planner.
    Planning,
    /// One batch end-to-end to validate the plan.
    Pilot,
    /// Remaining batches.
    Migration,
    /// Test translation and characterization.
    Integration,
    /// Adversarial review: critic.
    Hardening,
    /// Toolchain rerun as the only scorer.
    FinalValidation,
    /// Run finished.
    Done,
}

/// The live run state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// Current phase.
    pub phase: Phase,
    /// Delegations completed since the phase began. The `done` decision may
    /// advance only when this is at least one: a phase advances on evidence,
    /// not on a bare orchestrator claim.
    #[serde(default)]
    pub phase_delegations: u64,
    /// Task the orchestrator currently delegates on, when any.
    #[serde(default)]
    pub current_task: Option<String>,
    /// Batch currently migrating, when any.
    #[serde(default)]
    pub current_batch: Option<String>,
    /// Model driving the current work (router may promote).
    pub current_model: String,
    /// RFC 3339 UTC timestamp of the last write.
    pub updated_at: String,
    /// Last transition reason (human-readable, STE-clean).
    #[serde(default)]
    pub last_transition: String,
}

/// Write `state.json` into `dir`, creating the directory.
pub fn write(dir: &Path, state: &State) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let text = serde_json::to_string_pretty(state)?;
    std::fs::write(dir.join("state.json"), text).with_context(|| format!("write state.json in {}", dir.display()))
}

/// Read `state.json` from `dir`. `None` before the first write.
pub fn read(dir: &Path) -> anyhow::Result<Option<State>> {
    match std::fs::read_to_string(dir.join("state.json")) {
        Ok(text) => Ok(Some(serde_json::from_str(&text)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
