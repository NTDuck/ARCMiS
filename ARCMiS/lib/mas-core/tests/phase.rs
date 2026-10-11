//! Integration tests for the phase machine regression edges.

use mas_core::phase::Evidence;
use mas_core::phase::Phase;
use mas_core::phase::PhaseMachine;

#[test]
fn forward_walk_covers_every_phase_in_order() {
    let mut machine = PhaseMachine::new();
    let mut current = Phase::Preflight;
    while current != Phase::Done {
        let next = current.forward_next();
        machine
            .try_advance(next, "walk", Some(Evidence::CompletedDispatch))
            .unwrap_or_else(|error| panic!("advance {current:?} to {next:?}: {error}"));
        current = next;
    }
    assert_eq!(machine.phase(), Phase::Done);
}

#[test]
fn missing_evidence_blocks_advance() {
    let mut machine = PhaseMachine::new();
    let blocked = machine.try_advance(Phase::Discovery, "no evidence", None);
    assert!(blocked.is_err());
    assert_eq!(machine.phase(), Phase::Preflight);
}

#[test]
fn unmet_deliverable_gate_blocks_advance() {
    let mut machine = PhaseMachine::new();
    machine.accept(Phase::Pilot, "test setup");
    let blocked = machine.try_advance(
        Phase::Migration,
        "thin evidence",
        Some(Evidence::Deliverables {
            published: 0,
            minimum: 1,
        }),
    );
    assert!(blocked.is_err());
    assert_eq!(machine.phase(), Phase::Pilot);
}

#[test]
fn illegal_transition_is_rejected() {
    let mut machine = PhaseMachine::new();
    assert!(!PhaseMachine::is_legal(Phase::Preflight, Phase::Migration));
    let blocked = machine.try_advance(Phase::Integration, "skip", Some(Evidence::CompletedDispatch));
    assert!(blocked.is_err());
}

#[test]
fn regression_edges_match_retired_state_machine() {
    use Phase::*;
    for (from, to) in [
        (Pilot, Discovery),
        (Migration, Discovery),
        (Integration, Migration),
        (Hardening, Integration),
        (FinalValidation, Integration),
    ] {
        assert!(PhaseMachine::is_legal(from, to), "{from:?} to {to:?}");
    }
}

#[test]
fn regression_targets_cover_repairable_phases_only() {
    assert_eq!(Phase::regression_target(Phase::Pilot), Some(Phase::Discovery));
    assert_eq!(Phase::regression_target(Phase::Migration), Some(Phase::Discovery));
    assert_eq!(Phase::regression_target(Phase::Hardening), Some(Phase::Integration));
    assert_eq!(Phase::regression_target(Phase::Preflight), None);
    assert_eq!(Phase::regression_target(Phase::Done), None);
}

#[test]
fn history_records_every_accepted_transition() {
    let mut machine = PhaseMachine::new();
    machine.accept(Phase::Discovery, "one");
    machine.accept(Phase::Contract, "two");
    assert_eq!(machine.history.len(), 2);
    assert_eq!(machine.history[0].from, Phase::Preflight);
    assert_eq!(machine.history[1].to, Phase::Contract);
}
