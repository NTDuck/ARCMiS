//! Round-level laya triage (ADR 0028). The same trained
//! `agent_trace_observability` workflow that classifies one member dispatch
//! (ADR 0027) also consults on one closed or in-flight round: the judge sees
//! the round's evidence as the state and answers the five trained questions
//! about it. The question map and the prediction mapping stay byte-identical
//! with the dispatch path — the checkpoint is fine-tuned on exactly this
//! schema, and an off-workflow schema produces base accuracy (~0.36).
//!
//! The state maps the round semantics into the trained field names:
//! `phase` carries the phase reached, `passed` the completion flag, and the
//! output-snippet field carries the stop reason plus the round counters.
//! `role` keeps the trained semantics only loosely (it names the run's
//! subject), which is a documented fidelity limit, not a tuning target.

use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use laya::Agent;
use serde_json::Value;

use crate::jev_triage::TriageConsultation;
use crate::round_triage::evidence_io::aggregate_fields;
use crate::round_triage::evidence_io::count_action;
use crate::round_triage::evidence_io::count_kind;
use crate::round_triage::evidence_io::read_failure_causes;

/// Longest evidence text the round state carries. The stop reason and the
/// counter summary read as one short clause. A few hundred characters cover
/// it (the dispatch path caps at 2000, the round evidence is smaller).
const EVIDENCE_CAP_CHARS: usize = 2000;

/// The round evidence the judge consults on. Built from on-disk artifacts
/// (`result/`, `run/ledgers/`, `traces/`), never from guesses.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundEvidence {
    /// Phase the round reached (the state machine's final phase name).
    pub phase_reached: String,
    /// Whether the round's aggregate reports completion (phase Done).
    pub completed: bool,
    /// Why the run stopped, when it stopped early. Empty when the run
    /// reached Done or is in flight.
    pub stop_reason: String,
    /// Consecutive stalled rounds the ledger recorded.
    pub stalled_rounds: usize,
    /// Round counter from the aggregate (0 when absent).
    pub rounds: usize,
    /// Delegation count from the aggregate (0 when absent).
    pub delegations: usize,
    /// Delegations completed over delegations attempted.
    pub tasks_done: usize,
    pub tasks_total: usize,
    /// Model calls that died on the turn budget (`MaxTurnsError`).
    pub max_turns_deaths: usize,
    /// Model calls that died on the output budget (finish_reason=Length).
    pub output_cap_deaths: usize,
    /// Escalation decisions the orchestrator recorded.
    pub escalations: usize,
    /// Model failures with `context_length_exceeded`.
    pub context_length_events: usize,
    /// Failure rows the failure ledger recorded (all categories).
    pub failures: usize,
    /// Wall seconds of the round when the aggregate reports them.
    pub wall_seconds: Option<u64>,
}

impl RoundEvidence {
    /// Scan one run directory's ledgers. Reads `run/ledgers/*`. Aggregate
    /// fields (phase, completion, stop reason, counters, wall) come from the
    /// caller, which owns the run state. `load` fills them from a closed
    /// round's `result/aggregate.yml` instead.
    ///
    /// # Errors
    /// When the run dir has no `run/ledgers` tree: there is nothing to scan.
    pub fn scan(run_dir: &Path) -> anyhow::Result<Self> {
        let ledgers = run_dir.join("ledgers");
        anyhow::ensure!(ledgers.is_dir(), "no ledgers under {}", run_dir.display());
        let failures = read_failure_causes(&ledgers.join("failures.jsonl"))?;
        let stalled_rounds = count_kind(&ledgers.join("observations.jsonl"), "round_stalled")?;
        let escalations = count_action(&ledgers.join("decisions.jsonl"), "escalate")?;
        Ok(Self {
            max_turns_deaths: failures.iter().filter(|cause| cause.contains("MaxTurnsError")).count(),
            output_cap_deaths: failures.iter().filter(|cause| cause.contains("finish_reason=Length")).count(),
            context_length_events: failures.iter().filter(|cause| cause.contains("context_length_exceeded")).count(),
            failures: failures.len(),
            stalled_rounds,
            escalations,
            completed: false,
            phase_reached: String::new(),
            stop_reason: String::new(),
            rounds: 0,
            delegations: 0,
            tasks_done: 0,
            tasks_total: 0,
            wall_seconds: None,
        })
    }

    /// Load the evidence of one round directory. Reads `result/aggregate.yml`
    /// and `run/ledgers/*`. A missing aggregate (round still in flight)
    /// reports in-flight fields as zeros. The caller decides whether that
    /// matters.
    ///
    /// # Errors
    /// When the round dir has no `run/` tree at all: there is nothing to
    /// triage.
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let run_dir = dir.join("run");
        let ledgers = run_dir.join("ledgers");
        anyhow::ensure!(ledgers.is_dir(), "no run ledgers under {}", dir.display());
        let failures = read_failure_causes(&ledgers.join("failures.jsonl"))?;
        let stalled_rounds = count_kind(&ledgers.join("observations.jsonl"), "round_stalled")?;
        let escalations = count_action(&ledgers.join("decisions.jsonl"), "escalate")?;
        let aggregate = aggregate_fields(&dir.join("result").join("aggregate.yml"))?;
        Ok(Self {
            max_turns_deaths: failures.iter().filter(|cause| cause.contains("MaxTurnsError")).count(),
            output_cap_deaths: failures.iter().filter(|cause| cause.contains("finish_reason=Length")).count(),
            context_length_events: failures.iter().filter(|cause| cause.contains("context_length_exceeded")).count(),
            failures: failures.len(),
            stalled_rounds,
            escalations,
            completed: aggregate.completed,
            phase_reached: aggregate.final_phase,
            stop_reason: aggregate.stop_reason,
            rounds: aggregate.rounds,
            delegations: aggregate.delegations,
            tasks_done: aggregate.tasks_done,
            tasks_total: aggregate.tasks_total,
            wall_seconds: aggregate.wall_seconds,
        })
    }

    /// The one-line evidence text the state carries in the output field.
    /// Counters only: no task text, no source code.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut parts = vec![format!(
            "phase {} tasks {}/{} rounds {} stalled {}",
            self.phase_reached, self.tasks_done, self.tasks_total, self.rounds, self.stalled_rounds
        )];
        if !self.stop_reason.is_empty() {
            parts.push(format!("stop: {}", self.stop_reason));
        }
        if self.max_turns_deaths > 0 {
            parts.push(format!("{} max-turns deaths", self.max_turns_deaths));
        }
        if self.output_cap_deaths > 0 {
            parts.push(format!("{} output-cap deaths", self.output_cap_deaths));
        }
        if self.context_length_events > 0 {
            parts.push(format!("{} context-length failures", self.context_length_events));
        }
        if self.escalations > 0 {
            parts.push(format!("{} escalations", self.escalations));
        }
        if self.failures > 0 {
            parts.push(format!("{} ledger failures", self.failures));
        }
        if let Some(wall) = self.wall_seconds {
            parts.push(format!("wall {wall}s"));
        }
        parts.join("; ")
    }
}

/// The laya-backed round triage. Loads the checkpoint once and consults it
/// per round. Policy gating lives in `policy` (pure, testable without a
/// checkpoint).
#[derive(Clone)]
pub struct RoundTriage {
    agent: Option<Arc<Agent>>,
    confidence_threshold: f64,
    policy: agents::TriagePolicy,
}

impl RoundTriage {
    /// Build from the dispatch-triage config plus the round policy knob.
    /// A disabled section, an empty checkpoint path, or a failed load keeps
    /// the consult off (`Fallback` forever). The policy still applies to
    /// constructed verdicts, so tests exercise it without a checkpoint.
    #[must_use]
    pub fn from_config(config: &agents::util::config::JevTriageConfig) -> Self {
        let agent = if config.enabled && !config.checkpoint.is_empty() {
            match Self::load(&config.checkpoint) {
                Ok(agent) => Some(agent),
                Err(error) => {
                    tracing::warn!(error = %error, "round triage load failed. Round triage stays off");
                    None
                },
            }
        } else {
            None
        };
        Self {
            agent,
            confidence_threshold: config.confidence_threshold,
            policy: config.policy,
        }
    }

    fn load(checkpoint: &str) -> anyhow::Result<Arc<Agent>> {
        let path = Path::new(checkpoint);
        anyhow::ensure!(path.is_dir(), "round triage checkpoint is not a directory: {checkpoint}");
        Agent::from_dir(path).map(Arc::new).context("laya checkpoint load failed")
    }

    /// Whether a checkpoint loaded.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.agent.is_some()
    }

    /// The configured policy.
    #[must_use]
    pub fn policy(&self) -> agents::TriagePolicy {
        self.policy
    }

    /// Consult the judge on one round's evidence. Never fails: any error
    /// inside laya falls back. One predict call answers all five questions
    /// in one forward pass.
    pub fn consult_round(&self, evidence: &RoundEvidence) -> TriageConsultation {
        let Some(agent) = &self.agent else {
            return TriageConsultation::Fallback;
        };
        let state = round_state(evidence);
        agent
            .predict(&state, &crate::jev_triage::workflow_questions())
            .context("round triage predict failed")
            .and_then(|prediction| crate::jev_triage::map_prediction(&prediction))
            .map_or(TriageConsultation::Fallback, |verdict| {
                if verdict.confidence >= self.confidence_threshold {
                    TriageConsultation::Decided {
                        verdict,
                    }
                } else {
                    tracing::info!(
                        confidence = verdict.confidence,
                        threshold = self.confidence_threshold,
                        "round triage verdict below the confidence threshold; fallback"
                    );
                    TriageConsultation::Fallback
                }
            })
    }

    /// Apply the policy to one consultation. `Observe` keeps the verdict a
    /// ledger row only. `Enforce` turns a confident stop-or-harmful verdict
    /// into an early end (the caller reads `RoundAction::Stop`).
    #[must_use]
    pub fn gate(&self, consultation: &TriageConsultation) -> RoundAction {
        crate::round_triage::policy::gate(self.policy, consultation)
    }
}

/// The round state the judge sees, mapped into the trained field names.
/// `role` carries the run subject, `phase` the phase reached, `passed` the
/// completion flag, and the output field the stop reason plus the counters.
fn round_state(evidence: &RoundEvidence) -> Value {
    let summary: String = evidence.summary().chars().take(EVIDENCE_CAP_CHARS).collect();
    serde_json::json!({
        "role": "migration-round",
        "phase": evidence.phase_reached,
        "passed": evidence.completed,
        "output": summary,
    })
}

pub mod evidence_io;
pub mod policy;

pub use policy::RoundAction;

#[cfg(test)]
mod evidence_tests;

#[cfg(test)]
mod state_tests;
