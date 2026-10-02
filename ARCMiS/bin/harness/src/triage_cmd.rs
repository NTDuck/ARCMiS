//! The `harness triage` subcommand (ADR 0028): consult the laya round
//! judge over one experiment directory's real ledgers, emit one JSON line
//! per consult through the cli_sink pattern. Works on closed rounds. The
//! CLI is read-only and never writes to the ledgers it scans.

use std::path::Path;
use std::path::PathBuf;

use agents::util::config::JevTriageConfig;
use orchestrator::RoundEvidence;
use orchestrator::RoundTriage;
use orchestrator::TriageConsultation;

/// One triage subcommand invocation.
pub struct TriageInvocation {
    /// Experiment directory (closed round or in-flight run).
    pub experiment_dir: PathBuf,
    /// Checkpoint path override. Empty reads the experiment config's
    /// `mas.jev_triage.checkpoint`.
    pub checkpoint: Option<String>,
}

/// Parse `triage <experiment-dir> [--checkpoint <path>]`.
pub fn parse(args: &[String]) -> anyhow::Result<TriageInvocation> {
    let mut dir = None;
    let mut checkpoint = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--checkpoint" => {
                checkpoint =
                    Some(args.get(index + 1).ok_or_else(|| anyhow::anyhow!("--checkpoint needs a path"))?.to_owned());
                index += 2;
            },
            other if other.starts_with("--") => anyhow::bail!("unknown triage flag {other}; expected --checkpoint"),
            other => {
                anyhow::ensure!(dir.is_none(), "triage takes one experiment directory");
                dir = Some(PathBuf::from(other));
                index += 1;
            },
        }
    }
    let experiment_dir = dir.ok_or_else(|| anyhow::anyhow!("triage needs an experiment directory"))?;
    Ok(TriageInvocation {
        experiment_dir,
        checkpoint,
    })
}

/// Run the triage subcommand: load the checkpoint once, consult over the
/// directory's real evidence, emit one JSON line.
pub fn run(invocation: &TriageInvocation) -> anyhow::Result<()> {
    let dir = &invocation.experiment_dir;
    let evidence = RoundEvidence::load(dir)?;
    // The config in the experiment dir owns the checkpoint. --checkpoint
    // overrides it. A missing config keeps the default checkpoint empty,
    // so the caller must name one: no hardcoded paths here.
    let mut config = JevTriageConfig::default();
    let config_path = dir.join("config.yml");
    if config_path.is_file() {
        let loaded = agents::Config::load(&config_path)?;
        config = loaded.mas.jev_triage;
    }
    if let Some(path) = &invocation.checkpoint {
        config.checkpoint = path.clone();
    }
    config.enabled = true;
    if config.checkpoint.is_empty() {
        anyhow::bail!(
            "no triage checkpoint: pass --checkpoint or set mas.jev_triage.checkpoint in {}",
            config_path.display()
        );
    }
    let triage = RoundTriage::from_config(&config);
    anyhow::ensure!(triage.is_enabled(), "triage checkpoint failed to load: {}", config.checkpoint);
    let consultation = triage.consult_round(&evidence);

    // Recorded per-dispatch triage rows already in the ledger, when the
    // round ran instrumented. Zero rows means uninstrumented. The CLI
    // never fabricates verdicts for those.
    let recorded = recorded_triage_rows(dir)?;

    let mut line = serde_json::json!({
        "dir": dir.display().to_string(),
        "evidence": evidence_summary(&evidence),
        "policy": format!("{:?}", config.policy).to_lowercase(),
        "recorded_dispatch_triage_rows": recorded,
    });
    match &consultation {
        TriageConsultation::Decided {
            verdict,
        } => {
            line["verdict"] = serde_json::json!({
                "outcome": verdict.outcome,
                "action": verdict.action,
                "needs_review": verdict.needs_review,
                "risk": verdict.risk,
                "urgency": verdict.urgency,
                "confidence": verdict.confidence,
            });
            line["gate"] = serde_json::json!(format!("{:?}", triage.gate(&consultation)).to_lowercase());
        },
        TriageConsultation::Fallback => {
            line["fallback"] = serde_json::json!({
                "reason": "below confidence threshold, disabled judge, or inference error",
            });
        },
    }
    crate::cli_sink::emit("triage", &line.to_string());
    Ok(())
}

/// Recorded `jev_triage` observation rows in the round's ledger.
fn recorded_triage_rows(dir: &Path) -> anyhow::Result<usize> {
    let path = dir.join("run").join("ledgers").join("observations.jsonl");
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    Ok(text
        .lines()
        .filter(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .and_then(|row| row.get("kind").and_then(serde_json::Value::as_str).map(str::to_owned))
                == Some("jev_triage".to_owned())
        })
        .count())
}

/// Flat evidence map for the JSON line.
#[allow(clippy::too_many_lines)]
fn evidence_summary(evidence: &RoundEvidence) -> serde_json::Value {
    serde_json::json!({
        "phase_reached": evidence.phase_reached,
        "completed": evidence.completed,
        "stop_reason": evidence.stop_reason,
        "stalled_rounds": evidence.stalled_rounds,
        "rounds": evidence.rounds,
        "delegations": evidence.delegations,
        "tasks_done": evidence.tasks_done,
        "tasks_total": evidence.tasks_total,
        "max_turns_deaths": evidence.max_turns_deaths,
        "output_cap_deaths": evidence.output_cap_deaths,
        "escalations": evidence.escalations,
        "context_length_events": evidence.context_length_events,
        "failures": evidence.failures,
        "wall_seconds": evidence.wall_seconds,
        "summary": evidence.summary(),
    })
}
