//! Policy gating over round-triage verdicts (ADR 0028). Pure logic: no
//! laya and I/O, testable with hand-constructed verdicts. The observe /
//! enforce knob itself serializes in the agents config crate; this module
//! owns the gate.

use agents::TriagePolicy;

use crate::jev_triage::TriageConsultation;
use crate::jev_triage::TriageVerdict;

/// The action one consultation triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundAction {
    /// Continue the loop. Every `Fallback` and every `Observe` verdict.
    Continue,
    /// End the batch or round early and ledger a triage failure row.
    Stop,
}

/// Gate one consultation. `Fallback` and below-threshold verdicts never
/// stop: only a confident `Decided` verdict with the trained `action=stop`
/// label or the `outcome=harmful` label does, and only under `Enforce`.
#[must_use]
pub fn gate(policy: TriagePolicy, consultation: &TriageConsultation) -> RoundAction {
    match policy {
        TriagePolicy::Observe => RoundAction::Continue,
        TriagePolicy::Enforce => match consultation {
            TriageConsultation::Decided {
                verdict,
            } if triage_says_stop(verdict) => RoundAction::Stop,
            _ => RoundAction::Continue,
        },
    }
}

/// Whether a decided verdict carries the trained stop signal: the trained
/// `stop` action label or the trained `harmful` outcome label.
#[must_use]
pub fn triage_says_stop(verdict: &TriageVerdict) -> bool {
    verdict.action == "stop" || verdict.outcome == "harmful"
}

#[cfg(test)]
mod tests {
    use agents::TriagePolicy;

    use super::gate;
    use super::RoundAction;
    use crate::jev_triage::TriageConsultation;
    use crate::jev_triage::TriageVerdict;

    fn verdict(outcome: &str, action: &str, confidence: f64) -> TriageVerdict {
        TriageVerdict {
            outcome: outcome.to_owned(),
            needs_review: false,
            risk: 1,
            urgency: 1,
            action: action.to_owned(),
            confidence,
        }
    }

    fn decided(outcome: &str, action: &str, confidence: f64) -> TriageConsultation {
        TriageConsultation::Decided {
            verdict: verdict(outcome, action, confidence),
        }
    }

    #[test]
    fn enforce_stops_on_a_confident_stop_action() {
        let policy = TriagePolicy::Enforce;
        assert_eq!(gate(policy, &decided("failure", "stop", 0.95)), RoundAction::Stop);
    }

    #[test]
    fn enforce_stops_on_a_harmful_outcome() {
        let policy = TriagePolicy::Enforce;
        assert_eq!(gate(policy, &decided("harmful", "continue", 0.9)), RoundAction::Stop);
    }

    #[test]
    fn enforce_continues_on_a_stop_verdict_below_the_threshold() {
        // A Fallback is a below-threshold Decided by construction: the
        // consult already gated it, so the policy sees the fallback shape.
        let policy = TriagePolicy::Enforce;
        assert_eq!(gate(policy, &TriageConsultation::Fallback), RoundAction::Continue);
    }

    #[test]
    fn enforce_continues_on_continue_and_observe_verdicts() {
        let policy = TriagePolicy::Enforce;
        assert_eq!(gate(policy, &decided("success", "continue", 0.95)), RoundAction::Continue);
        assert_eq!(gate(policy, &decided("partial", "observe", 0.9)), RoundAction::Continue);
    }

    #[test]
    fn observe_never_stops() {
        let policy = TriagePolicy::Observe;
        assert_eq!(gate(policy, &decided("failure", "stop", 0.99)), RoundAction::Continue);
        assert_eq!(gate(policy, &decided("harmful", "stop", 0.99)), RoundAction::Continue);
        assert_eq!(gate(policy, &TriageConsultation::Fallback), RoundAction::Continue);
    }

    #[test]
    fn the_stop_signal_is_the_label_conjunction() {
        assert!(super::triage_says_stop(&verdict("failure", "stop", 0.9)));
        assert!(super::triage_says_stop(&verdict("harmful", "continue", 0.9)));
        assert!(!super::triage_says_stop(&verdict("failure", "human_review", 0.9)));
        assert!(!super::triage_says_stop(&verdict("success", "continue", 0.9)));
    }
}
