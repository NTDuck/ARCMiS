//! Experiment: write the meta-harness manifest pre-run per ADR 0020, and the
//! result aggregate post-run. One experiment = one output directory under
//! `.artifacts/experiments/<id>/`.

use anyhow::Context as _;
use blackboard::Manifest;

/// Write `manifest.json` into the experiment directory before the run.
pub fn write_pre_run(dir: &std::path::Path, manifest: &Manifest) -> anyhow::Result<()> {
    blackboard::manifest::write(dir, manifest)?;
    tracing::info!(dir = %dir.display(), harness_id = %manifest.harness_id, "experiment manifest written");
    Ok(())
}

/// Write `result/aggregate.yml` after the run: pass/fail per phase, task
/// counts, and the token estimate.
pub fn write_result(dir: &std::path::Path, aggregate: &Aggregate) -> anyhow::Result<()> {
    let result_dir = dir.join("result");
    std::fs::create_dir_all(&result_dir)?;
    let text = serde_yaml::to_string(aggregate)?;
    std::fs::write(result_dir.join("aggregate.yml"), text)
        .with_context(|| format!("write aggregate.yml in {}", result_dir.display()))
}

/// Post-run aggregate.
#[derive(Debug, serde::Serialize)]
pub struct Aggregate {
    /// Experiment id.
    pub harness_id: String,
    /// Final phase reached.
    pub final_phase: String,
    /// Whether the run reached `Done`.
    pub completed: bool,
    /// Manager rounds executed.
    pub rounds: usize,
    /// Delegations executed.
    pub delegations: usize,
    /// Tasks done / total.
    pub tasks_done: usize,
    pub tasks_total: usize,
    /// Why the run stopped, when it did not reach Done.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Wall-clock seconds.
    pub duration_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrips_through_experiment_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest = Manifest {
            harness_id: "exp-test-1".into(),
            method: "mas".into(),
            model: "test".into(),
            budgets: blackboard::Budgets {
                manager_turns: 20,
                worker_turns: 40,
                max_rounds: 60,
            },
            config_path: "assets/configs/mas/config.yml".into(),
            problem_set: "test".into(),
            source_language: "c".into(),
            target_language: "rust".into(),
            git_revision: String::new(),
            parents: Vec::new(),
            hypothesis: String::new(),
            created_at: "2026-09-24T00:00:00Z".into(),
        };
        write_pre_run(dir.path(), &manifest).expect("write manifest");
        let read_back = blackboard::manifest::read(dir.path()).expect("read manifest");
        assert_eq!(read_back.harness_id, "exp-test-1");
        assert_eq!(read_back.method, "mas");
    }
}
