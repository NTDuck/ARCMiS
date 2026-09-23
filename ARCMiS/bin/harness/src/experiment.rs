//! Experiment artifact construction.
//!
//! One migration run with an experiment directory writes its manifest
//! before execution and its per-problem record after execution. The
//! directory layout follows ADR 0020: `manifest.json`, `result/`,
//! `traces/turns.jsonl`, and the produced workspace.

use agents::Config;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;
use std::process::Command;

/// One experiment manifest. The proposer reads this file first.
#[derive(Serialize)]
struct Manifest {
    candidate_id: String,
    method: String,
    model: String,
    budgets: Budgets,
    config_path: String,
    problem_root: String,
    target_language: String,
    git_revision: String,
    parents: Vec<String>,
    hypothesis: String,
    created_at: String,
}

/// Model and turn budgets copied from the config.
#[derive(Serialize)]
struct Budgets {
    max_turns: usize,
    num_ctx: u64,
    max_output_tokens: u64,
    max_retries: u32,
    temperature: f64,
}

/// One problem record inside the per-problem file.
#[derive(Serialize)]
struct ProblemRecord {
    problem: String,
    success: bool,
    stage: &'static str,
    tests_passed: u32,
    tests_failed: u32,
}

/// Derive the candidate id from the experiment directory name.
fn candidate_id(dir: &Path) -> String {
    dir.file_name().and_then(|name| name.to_str()).unwrap_or("candidate").to_owned()
}

/// Read the short git revision. An absent git returns `unknown`.
fn git_revision() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Current UTC timestamp in the compact `YYYYMMDDTHHMMSSZ` form.
fn timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::macros::format_description!("[year][month][day]T[hour][minute][second]Z"))
        .unwrap_or_else(|_| "unknown".to_owned())
}

/// Write the manifest before the run starts. The manifest declares what
/// the candidate runs, never what it scored.
pub fn write_manifest(dir: &Path, config: &Config, config_path: &Path, method: &str) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("experiment dir create failed at {}", dir.display()))?;
    let manifest = Manifest {
        candidate_id: candidate_id(dir),
        // The --method flag selects the pipeline; the harness logs the
        // resolved name here so the survey artifacts stay self-describing.
        method: method.to_owned(),
        model: config.run.model.clone(),
        budgets: Budgets {
            max_turns: config.run.max_turns,
            num_ctx: config.run.num_ctx,
            max_output_tokens: config.run.max_output_tokens,
            max_retries: config.run.max_retries,
            temperature: config.run.temperature,
        },
        config_path: config_path.display().to_string(),
        problem_root: config.source.root.display().to_string(),
        target_language: config.source.target.language.clone(),
        git_revision: git_revision(),
        parents: Vec::new(),
        hypothesis: String::new(),
        created_at: timestamp(),
    };
    let path = dir.join("manifest.json");
    std::fs::write(&path, serde_json::to_string_pretty(&manifest)?)
        .with_context(|| format!("manifest write failed at {}", path.display()))?;
    tracing::info!(manifest = %path.display(), "experiment manifest written");
    Ok(())
}

/// Score the produced workspace with the recorded toolchain. The
/// evaluator reruns `cargo build` and the configured test command, so
/// the score reflects the toolchain, not the agent's self-report.
fn evaluate_workspace(workspace: &Path, test_command: &str) -> Result<ProblemRecord> {
    let mut record = ProblemRecord {
        problem: "workspace".to_owned(),
        success: false,
        stage: "evaluate",
        tests_passed: 0,
        tests_failed: 0,
    };
    let build = Command::new("cargo")
        .args(["build"])
        .current_dir(workspace)
        .output()
        .context("workspace cargo build failed to start")?;
    if !build.status.success() {
        return Ok(record);
    }
    let mut parts = test_command.split_whitespace();
    let program = parts.next().context("empty test command")?;
    let test = Command::new(program)
        .args(parts)
        .current_dir(workspace)
        .output()
        .context("workspace test command failed to start")?;
    let text = format!("{}{}", String::from_utf8_lossy(&test.stdout), String::from_utf8_lossy(&test.stderr));
    record.tests_passed = passed_count(&text);
    record.tests_failed = failed_count(&text);
    record.success = test.status.success();
    Ok(record)
}

/// Sum the `N passed` markers of one cargo test output.
fn passed_count(text: &str) -> u32 {
    count_marker(text, " passed")
}

/// Count `<n> passed` occurrences. Cargo writes the number and the
/// marker as two whitespace-separated tokens, often with a trailing
/// semicolon (`1 passed;`), so scan token pairs instead of suffixes.
fn count_marker(text: &str, marker: &str) -> u32 {
    let marker_word = marker.trim_start();
    let mut total = 0;
    let mut previous: Option<&str> = None;
    for token in text.split_whitespace() {
        if let Some(current) = previous.take() {
            let word = token.trim_end_matches(|c: char| !c.is_ascii_alphabetic());
            if word == marker_word {
                if let Ok(value) = current.parse::<u32>() {
                    total += value;
                }
            }
        }
        previous = Some(token);
    }
    total
}

/// Count `N failed` occurrences.
fn failed_count(text: &str) -> u32 {
    count_marker(text, " failed")
}

/// Write the per-problem record after the run. The toolchain rerun
/// decides success, never the agent's self-report.
pub fn write_per_problem(dir: &Path, config: &Config) -> Result<()> {
    let workspace = &config.output.dir;
    let toolchain = evaluate_workspace(workspace, &config.source.target.test_command)?;
    // The stage records which pipeline phase last touched the workspace:
    // the monolith validator always reaches evaluation, the recode and
    // ledger agents finish with their own typed report.
    let record = toolchain;
    let result_dir = dir.join("result");
    std::fs::create_dir_all(&result_dir)?;
    let path = result_dir.join("per_problem.json");
    std::fs::write(&path, serde_json::to_string_pretty(&vec![record])?)
        .with_context(|| format!("per-problem write failed at {}", path.display()))?;
    Ok(())
}

/// Copy the aggregate result yaml the run produced into the experiment
/// result directory, when one exists.
pub fn copy_result(dir: &Path, config: &Config) -> Result<()> {
    let source = config.output.dir.join(".ARCMiS").join("result");
    if !source.is_dir() {
        return Ok(());
    }
    let result_dir = dir.join("result");
    std::fs::create_dir_all(&result_dir)?;
    for entry in std::fs::read_dir(&source)?.flatten() {
        let target = result_dir.join(entry.file_name());
        std::fs::copy(entry.path(), &target)?;
    }
    Ok(())
}
