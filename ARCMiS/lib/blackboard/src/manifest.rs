//! Run manifest: the identity of one harness run. Written pre-run per ADR
//! 0020 so meta-harness tooling can classify the experiment directory before
//! the run produces anything.

use std::path::Path;

use anyhow::Context as _;
use serde::Deserialize;
use serde::Serialize;

/// Run budgets echoed from the config for provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budgets {
    /// Orchestrator turn budget.
    pub orchestrator_turns: usize,
    /// Worker turn budget per delegation.
    pub worker_turns: usize,
    /// Orchestrator-loop round ceiling.
    pub max_rounds: usize,
}

/// One harness run's manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Unique id: timestamp + counter.
    pub harness_id: String,
    /// Method tag; v1 is always `mas`.
    pub method: String,
    /// Model serving the agents.
    pub model: String,
    /// Budgets from the config.
    pub budgets: Budgets,
    /// Config file path relative to the workspace root.
    pub config_path: String,
    /// Problem set label (e.g. `GildedRose-Refactoring-Kata`).
    pub problem_set: String,
    /// Source language of the input codebase.
    pub source_language: String,
    /// Target language of the migration.
    pub target_language: String,
    /// Test command from the config's target section, when configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_command: Option<String>,
    /// Git revision of the harness at run start (empty when unavailable).
    pub git_revision: String,
    /// Parent candidate ids for meta-harness lineage.
    #[serde(default)]
    pub parents: Vec<String>,
    /// Hypothesis note for meta-harness experiments.
    #[serde(default)]
    pub hypothesis: String,
    /// RFC 3339 UTC creation timestamp.
    pub created_at: String,
}

/// Write `manifest.json` into `dir`, creating the directory.
pub fn write(dir: &Path, manifest: &Manifest) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let text = serde_json::to_string_pretty(manifest)?;
    std::fs::write(dir.join("manifest.json"), text).with_context(|| format!("write manifest.json in {}", dir.display()))
}

/// Read `manifest.json` from `dir`.
pub fn read(dir: &Path) -> anyhow::Result<Manifest> {
    let text = std::fs::read_to_string(dir.join("manifest.json"))?;
    serde_json::from_str(&text).map_err(Into::into)
}
