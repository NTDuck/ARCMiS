//! Phase state machine. Forward transitions are earned by phase exit
//! conditions; regressions (MIGRATION → DISCOVERY, INTEGRATION → MIGRATION)
//! are allowed and recorded. Anything else is a programming error.

use blackboard::Phase;
use serde::Serialize;

/// One attempted transition.
#[derive(Debug, Clone, Serialize)]
pub struct Transition {
    /// Phase before the transition.
    pub from: Phase,
    /// Phase after the transition.
    pub to: Phase,
    /// Why the transition fired (STE-clean).
    pub reason: String,
}

/// Outcome of asking the machine to advance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Advance {
    /// The machine moved to `Phase`.
    Moved(Phase),
    /// The exit condition for the current phase does not hold yet; stay.
    Hold(Phase),
}

/// Static order for progress reporting and forward-walk validation.
#[must_use]
pub fn forward_order() -> [Phase; 10] {
    [
        Phase::Preflight,
        Phase::Discovery,
        Phase::Contract,
        Phase::Planning,
        Phase::Pilot,
        Phase::Migration,
        Phase::Integration,
        Phase::Hardening,
        Phase::FinalValidation,
        Phase::Done,
    ]
}

/// Whether `to` is a legal successor of `from`.
#[must_use]
pub fn is_legal(from: Phase, to: Phase) -> bool {
    use Phase::*;
    match (from, to) {
        // Forward walk, one step.
        (Preflight, Discovery)
        | (Discovery, Contract)
        | (Contract, Planning)
        | (Planning, Pilot)
        | (Pilot, Migration)
        | (Migration, Integration)
        | (Integration, Hardening)
        | (Hardening, FinalValidation)
        | (FinalValidation, Done) => true,
        // Regressions on failure.
        (Pilot, Discovery) | (Migration, Discovery) => true,
        (Integration, Migration) => true,
        (Hardening, Integration) | (FinalValidation, Integration) => true,
        // Self-transitions record in-place phase events (e.g. batch loops).
        (Pilot, Pilot) | (Migration, Migration) | (Integration, Integration) => true,
        _ => false,
    }
}

/// Apply one transition; errors on an illegal move. Returns the record.
pub fn apply(state: &mut blackboard::State, to: Phase, reason: &str) -> anyhow::Result<Transition> {
    anyhow::ensure!(is_legal(state.phase, to), "illegal phase transition {:?} -> {:?}", state.phase, to);
    let record = Transition {
        from: state.phase,
        to,
        reason: reason.to_owned(),
    };
    state.phase = to;
    // The new phase starts with no completed work of its own.
    state.phase_delegations = 0;
    state.last_transition = reason.to_owned();
    Ok(record)
}

/// Position of one phase in the forward order (Done is last).
#[must_use]
pub fn progress(phase: Phase) -> usize {
    forward_order().iter().position(|step| *step == phase).unwrap_or(forward_order().len() - 1)
}

/// The forward successor of `phase`. `Done` is terminal and returns itself.
#[must_use]
pub fn next(phase: Phase) -> Phase {
    let order = forward_order();
    let at = progress(phase);
    order.into_iter().nth((at + 1).min(order.len() - 1)).unwrap_or(phase)
}
