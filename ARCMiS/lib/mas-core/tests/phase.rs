//! Integration tests for the phase machine regression edges.

// WHY the allow: the legacy walk tests exercise the deprecated variants
// to confirm backward compatibility. The new walk tests cover the
// fifteen-state forward path.
#![allow(deprecated)]

use mas_core::module_phase::ModulePhase;
use mas_core::module_phase::ModuleStateMachine;
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
fn forward_walk_from_init_to_done() {
    let mut machine = PhaseMachine::init(Phase::Init);
    let mut current = Phase::Init;
    while current != Phase::Done {
        let next = current.forward_next();
        machine
            .try_advance(next, "walk", Some(Evidence::CompletedDispatch))
            .unwrap_or_else(|error| panic!("advance {current:?} to {next:?}: {error}"));
        current = next;
    }
    assert_eq!(machine.phase(), Phase::Done);
    assert_eq!(machine.history.len(), 14);
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

#[test]
fn new_regression_edges_match_adr_0039() {
    use Phase::*;
    for (from, to) in [
        (Assess, Preflight),
        (Map, Assess),
        (ExtractRules, Map),
        (ReviewRules, ExtractRules),
        (DraftBrief, ReviewRules),
        (MigratePilot, DraftBrief),
        (VerifyPilot, MigratePilot),
        (MigrateBatches, VerifyPilot),
        (VerifyModule, MigrateBatches),
        (FinalVerify, VerifyModule),
        (Harden, FinalVerify),
    ] {
        assert!(PhaseMachine::is_legal(from, to), "{from:?} to {to:?}");
        assert_eq!(Phase::regression_target(from), Some(to));
    }
}

#[test]
fn module_forward_walk_covers_every_phase_in_order() {
    let mut machine = ModuleStateMachine::new(3);
    let mut current = ModulePhase::Planned;
    while current != ModulePhase::Proven {
        let next = current.forward_next();
        machine.try_advance(next, "walk").unwrap_or_else(|error| panic!("advance {current:?} to {next:?}: {error}"));
        current = next;
    }
    assert_eq!(machine.phase(), ModulePhase::Proven);
    assert!(machine.is_terminal());
    assert_eq!(machine.history().len(), 5);
}

#[test]
fn module_failure_transitions_are_legal() {
    use ModulePhase::*;
    for (from, to) in [
        (Migrating, Diagnosing),
        (Diagnosing, Repairing),
        (Repairing, Migrating),
        (Migrating, Escalated),
        (Verifying, Diagnosing),
        (Repairing, Verifying),
        (Verifying, Escalated),
    ] {
        assert!(ModuleStateMachine::is_legal(from, to), "{from:?} to {to:?}");
    }
}

#[test]
fn module_regression_edges_match_adr_0039() {
    use ModulePhase::*;
    for (from, to) in [
        (Baselined, Ready),
        (Migrating, Baselined),
        (Verifying, Migrating),
        (Diagnosing, Baselined),
        (Repairing, Diagnosing),
    ] {
        assert!(ModuleStateMachine::is_legal(from, to), "{from:?} to {to:?}");
    }
}

#[test]
fn module_escalated_is_terminal() {
    let mut machine = ModuleStateMachine::new(0);
    machine.try_advance(ModulePhase::Ready, "schedule").unwrap();
    machine.try_advance(ModulePhase::Baselined, "baseline").unwrap();
    machine.try_advance(ModulePhase::Migrating, "translate").unwrap();
    machine.try_advance(ModulePhase::Escalated, "budget exhausted").unwrap();
    assert_eq!(machine.phase(), ModulePhase::Escalated);
    assert!(machine.is_terminal());
    let blocked = machine.try_advance(ModulePhase::Diagnosing, "retry");
    assert!(blocked.is_err());
}

#[test]
fn module_escalation_requires_repair_budget() {
    let mut machine = ModuleStateMachine::new(2);
    machine.try_advance(ModulePhase::Ready, "schedule").unwrap();
    machine.try_advance(ModulePhase::Baselined, "baseline").unwrap();
    machine.try_advance(ModulePhase::Migrating, "translate").unwrap();
    let blocked = machine.try_advance(ModulePhase::Escalated, "too early");
    assert!(blocked.is_err());
    assert_eq!(machine.phase(), ModulePhase::Migrating);
}

#[test]
fn module_repairs_used_increments_on_each_repair() {
    let mut machine = ModuleStateMachine::new(3);
    machine.try_advance(ModulePhase::Ready, "schedule").unwrap();
    machine.try_advance(ModulePhase::Baselined, "baseline").unwrap();
    machine.try_advance(ModulePhase::Migrating, "translate").unwrap();
    assert_eq!(machine.repairs_used(), 0);
    machine.try_advance(ModulePhase::Diagnosing, "fail").unwrap();
    machine.try_advance(ModulePhase::Repairing, "fix").unwrap();
    assert_eq!(machine.repairs_used(), 1);
    machine.try_advance(ModulePhase::Migrating, "retry").unwrap();
    machine.try_advance(ModulePhase::Diagnosing, "fail").unwrap();
    machine.try_advance(ModulePhase::Repairing, "fix").unwrap();
    assert_eq!(machine.repairs_used(), 2);
}

#[test]
fn module_illegal_transition_is_rejected() {
    let mut machine = ModuleStateMachine::new(3);
    let blocked = machine.try_advance(ModulePhase::Proven, "skip");
    assert!(blocked.is_err());
    assert_eq!(machine.phase(), ModulePhase::Planned);
}
