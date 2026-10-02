//! Round-level triage wiring for the harness run loop (ADR 0028). Consults
//! the laya round judge after each orchestrator round and, under the
//! enforce policy, ends the run early on a confident stop-or-harmful
//! verdict. WHY: the per-dispatch triage (ADR 0027) is instrumentation
//! only; round-level enforce is the first policy surface, gated by the
//! `mas.jev_triage.policy` knob, default observe (zero behavior change).

use std::path::Path;

use blackboard::Ledger;
use orchestrator::RoundAction;
use orchestrator::RoundEvidence;
use orchestrator::RoundTriage;
use orchestrator::TriageConsultation;

/// Consult the round judge on the round just closed and ledger a
/// `jev_round_triage` observation on a decided verdict. Returns the policy
/// action: `Stop` means the caller ends the run early.
#[allow(clippy::too_many_arguments)]
pub fn consult_round(
    triage: &RoundTriage,
    ledger: &Ledger,
    run_dir: &Path,
    phase: &str,
    completed: bool,
    stop_reason: Option<&str>,
    wall_seconds: u64,
) -> RoundAction {
    let mut evidence = match RoundEvidence::scan(run_dir) {
        Ok(evidence) => evidence,
        Err(error) => {
            tracing::warn!(error = %error, "round evidence scan failed; round triage skipped");
            return RoundAction::Continue;
        },
    };
    evidence.phase_reached = phase.to_owned();
    evidence.completed = completed;
    evidence.stop_reason = stop_reason.unwrap_or_default().to_owned();
    evidence.wall_seconds = Some(wall_seconds);
    let consultation = triage.consult_round(&evidence);
    if let TriageConsultation::Decided {
        verdict,
    } = &consultation
    {
        let _ = ledger.append_observation(&blackboard::Observation {
            at: crate::now_rfc3339(),
            kind: "jev_round_triage".into(),
            detail: serde_json::json!({
                "phase": evidence.phase_reached,
                "completed": evidence.completed,
                "stop_reason": evidence.stop_reason,
                "stalled_rounds": evidence.stalled_rounds,
                "rounds": evidence.rounds,
                "tasks_done": evidence.tasks_done,
                "tasks_total": evidence.tasks_total,
                "max_turns_deaths": evidence.max_turns_deaths,
                "output_cap_deaths": evidence.output_cap_deaths,
                "escalations": evidence.escalations,
                "context_length_events": evidence.context_length_events,
                "failures": evidence.failures,
                "wall_seconds": evidence.wall_seconds,
                "outcome": verdict.outcome,
                "action": verdict.action,
                "needs_review": verdict.needs_review,
                "risk": verdict.risk,
                "urgency": verdict.urgency,
                "confidence": verdict.confidence,
            }),
        });
    }
    triage.gate(&consultation)
}
