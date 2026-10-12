//! Integration tests for the module-phase crate.

use mas_module_phase::ModulePhase;
use mas_module_phase::ModuleStateMachine;

#[test]
fn forward_walk_covers_every_phase_in_order() {
    let mut machine = ModuleStateMachine::new(2);
    let walk = [
        ModulePhase::Ready,
        ModulePhase::Baselined,
        ModulePhase::Migrating,
        ModulePhase::Verifying,
        ModulePhase::Proven,
    ];
    for next in walk {
        machine.try_advance(next, "gate met").expect("forward advance");
    }
    assert_eq!(machine.phase(), ModulePhase::Proven);
    assert!(machine.is_terminal());
}

#[test]
fn illegal_transition_is_rejected() {
    let mut machine = ModuleStateMachine::new(2);
    let result = machine.try_advance(ModulePhase::Proven, "skip");
    assert!(result.is_err());
}

#[test]
fn escalation_requires_repair_budget_exhausted() {
    let mut machine = ModuleStateMachine::new(2);
    machine.try_advance(ModulePhase::Ready, "ready").expect("ready");
    machine.try_advance(ModulePhase::Baselined, "baselined").expect("baselined");
    machine.try_advance(ModulePhase::Migrating, "migrating").expect("migrating");
    let result = machine.try_advance(ModulePhase::Escalated, "too early");
    assert!(result.is_err());
}

#[test]
fn repairs_used_increments_on_each_repair() {
    let mut machine = ModuleStateMachine::new(2);
    machine.try_advance(ModulePhase::Ready, "ready").expect("ready");
    machine.try_advance(ModulePhase::Baselined, "baselined").expect("baselined");
    machine.try_advance(ModulePhase::Migrating, "migrating").expect("migrating");
    machine.try_advance(ModulePhase::Diagnosing, "diagnose").expect("diagnose");
    machine.try_advance(ModulePhase::Repairing, "repair").expect("repair");
    assert_eq!(machine.repairs_used(), 1);
    machine.try_advance(ModulePhase::Migrating, "retry").expect("retry");
    machine.try_advance(ModulePhase::Diagnosing, "diagnose").expect("diagnose");
    machine.try_advance(ModulePhase::Repairing, "repair").expect("repair");
    assert_eq!(machine.repairs_used(), 2);
}
