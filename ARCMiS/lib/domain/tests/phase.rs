//! Integration tests for the phase module.

use mas_domain::Evidence;
use mas_domain::Phase;
use mas_domain::PhaseMachine;

#[test]
fn forward_walk_covers_every_phase_in_order() {
    let mut machine = PhaseMachine::init(Phase::Init);
    let walk = [
        Phase::Preflight,
        Phase::Assess,
        Phase::Map,
        Phase::ExtractRules,
        Phase::ReviewRules,
        Phase::DraftBrief,
        Phase::MigratePilot,
        Phase::VerifyPilot,
        Phase::MigrateBatches,
        Phase::VerifyModule,
        Phase::FinalVerify,
        Phase::Harden,
        Phase::HumanSignOff,
        Phase::Done,
    ];
    for next in walk {
        machine.try_advance(next, "gate met", Some(Evidence::CompletedDispatch)).expect("forward advance");
    }
    assert_eq!(machine.phase(), Phase::Done);
    assert_eq!(machine.history.len(), walk.len());
}

#[test]
fn illegal_transition_is_rejected() {
    let mut machine = PhaseMachine::init(Phase::Init);
    let result = machine.try_advance(Phase::Done, "skip", Some(Evidence::CompletedDispatch));
    assert!(result.is_err());
}

#[test]
fn unmet_deliverable_gate_blocks_advance() {
    let mut machine = PhaseMachine::init(Phase::Assess);
    let result = machine.try_advance(
        Phase::Map,
        "gate met",
        Some(Evidence::Deliverables {
            published: 0,
            minimum: 1,
        }),
    );
    assert!(result.is_err());
}

#[test]
fn regression_targets_cover_repairable_phases_only() {
    assert_eq!(Phase::regression_target(Phase::Assess), Some(Phase::Preflight));
    assert_eq!(Phase::regression_target(Phase::Map), Some(Phase::Assess));
    assert_eq!(Phase::regression_target(Phase::Init), None);
    assert_eq!(Phase::regression_target(Phase::Done), None);
}

#[test]
fn evidence_holds_checks_its_own_gate() {
    assert!(Evidence::CompletedDispatch.holds());
    assert!(Evidence::TestCommandGreen.holds());
    assert!(Evidence::Deliverables {
        published: 2,
        minimum: 1,
    }
    .holds());
    assert!(!Evidence::Deliverables {
        published: 0,
        minimum: 1,
    }
    .holds());
}
