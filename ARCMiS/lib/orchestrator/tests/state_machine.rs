//! State machine test: legal forward walks, legal regressions, illegal
//! jumps, progress ordering, and breaker trips.

use agents::util::config::MasConfig;
use agents::Role;
use blackboard::Phase;
use blackboard::State;
use orchestrator::breaker::Breaker;
use orchestrator::breaker::BreakerState;
use orchestrator::judges;
use orchestrator::router;
use orchestrator::state_machine;
use orchestrator::state_machine::is_legal;

fn state_at(phase: Phase) -> State {
    State {
        phase,
        phase_delegations: 0,
        current_task: None,
        current_batch: None,
        current_model: "test-model".into(),
        updated_at: "unix:0".into(),
        last_transition: String::new(),
    }
}

#[test]
fn forward_walk_is_legal_step_by_step() {
    let order = state_machine::forward_order();
    for window in order.windows(2) {
        assert!(is_legal(window[0], window[1]), "forward step {:?} -> {:?} must be legal", window[0], window[1]);
    }
    // Skipping one phase is illegal.
    assert!(!is_legal(Phase::Preflight, Phase::Planning));
    assert!(!is_legal(Phase::Discovery, Phase::Migration));
    assert!(!is_legal(Phase::Contract, Phase::Done));
    // Backwards without a regression path is illegal.
    assert!(!is_legal(Phase::Hardening, Phase::Planning));
    assert!(!is_legal(Phase::Done, Phase::Migration));
}

#[test]
fn regressions_and_self_loops_are_legal() {
    assert!(is_legal(Phase::Migration, Phase::Discovery));
    assert!(is_legal(Phase::Integration, Phase::Migration));
    assert!(is_legal(Phase::Hardening, Phase::Integration));
    assert!(is_legal(Phase::Migration, Phase::Migration));
    assert!(is_legal(Phase::Pilot, Phase::Pilot));
    // Regression application mutates the state and records the reason.
    let mut state = state_at(Phase::Integration);
    let record = state_machine::apply(&mut state, Phase::Migration, "tests failed").expect("legal regression");
    assert_eq!(record.from, Phase::Integration);
    assert_eq!(record.to, Phase::Migration);
    assert_eq!(state.phase, Phase::Migration);
    assert_eq!(state.last_transition, "tests failed");
    // Illegal transition errors.
    let mut state = state_at(Phase::Preflight);
    assert!(state_machine::apply(&mut state, Phase::Done, "skip").is_err());
    assert_eq!(state.phase, Phase::Preflight, "failed apply must not mutate");
}

#[test]
fn progress_orders_phases() {
    assert_eq!(state_machine::progress(Phase::Preflight), 0);
    assert_eq!(state_machine::progress(Phase::Migration), 5);
    assert_eq!(state_machine::progress(Phase::Done), 9);
}

#[test]
fn breaker_trips_on_repeated_failure_and_stagnation() {
    let mut config = MasConfig {
        max_repairs: 2,
        stagnation_rounds: 3,
        ..MasConfig::default()
    };

    let mut breaker = BreakerState::new();
    // Two failures with max_repairs=2 are still inside budget.
    breaker.record_failure();
    breaker.record_failure();
    assert_eq!(breaker.check(Phase::Migration, &config), Breaker::Continue);
    // The third consecutive failure trips.
    breaker.record_failure();
    assert!(matches!(breaker.check(Phase::Migration, &config), Breaker::Trip(_)));
    // Progress resets the failure count.
    breaker.record_progress();
    assert_eq!(breaker.check(Phase::Migration, &config), Breaker::Continue);

    // Stagnation: three stalled rounds trip.
    let mut breaker = BreakerState::new();
    breaker.record_stalled_round();
    breaker.record_stalled_round();
    assert_eq!(breaker.check(Phase::Planning, &config), Breaker::Continue);
    breaker.record_stalled_round();
    assert!(matches!(breaker.check(Phase::Planning, &config), Breaker::Trip(_)));
    breaker.record_productive_round();
    assert_eq!(breaker.check(Phase::Planning, &config), Breaker::Continue);

    // max_rounds=0 disables the budget path entirely (loose guard).
    config.max_rounds = 0;
    let mut breaker = BreakerState::new();
    breaker.add_tokens(1_000_000);
    assert_eq!(breaker.check(Phase::Discovery, &config), Breaker::Continue);
}

#[test]
fn router_keyword_pass_covers_role_verbs() {
    let cases = [
        ("diagnose the toolchain failure in build.rs", Role::FailureAnalyst),
        ("fix one diagnosed failure in src/lib.rs", Role::Repairer),
        ("repair the broken import", Role::Repairer),
        ("validate the translated batch-1 modules", Role::Validator),
        ("adversarial review of the finished migration", Role::Critic),
        ("produce the source map for the codebase", Role::Analyst),
        ("write the migration brief with gap decisions", Role::Architect),
        ("split the modules into dependency batches", Role::Planner),
        ("translate module src/parser.rs to Rust", Role::Translator),
        ("add characterization tests and run the test command", Role::Tester),
    ];
    for (description, expected) in cases {
        assert_eq!(router::route_by_keywords(description), Some(expected), "routing {description:?}");
    }
    // Unroutable text falls through to None.
    assert_eq!(router::route_by_keywords("do the thing"), None);
}

#[test]
fn judge_parsers_accept_contract_lines() {
    // Diagnosis.
    let diagnosis = judges::parse_diagnosis(
        "looked at the build.\nDIAGNOSIS: toolchain | missing serde feature in Cargo.toml | add serde with derive and \
         rebuild",
    )
    .expect("diagnosis line");
    assert_eq!(diagnosis.category, judges::FailureCategory::Toolchain);
    assert!(diagnosis.root_cause.contains("serde"));
    assert!(diagnosis.suggested_action.contains("rebuild"));
    // Invalid category rejected.
    assert!(judges::parse_diagnosis("DIAGNOSIS: bogus | x | y").is_none());

    // Verdicts.
    let (passed, reason) =
        judges::parse_verdict("notes...\nVALIDATION: pass | contracts hold", "VALIDATION").expect("verdict");
    assert!(passed);
    assert!(reason.contains("contracts"));
    let (passed, _) =
        judges::parse_verdict("CRITIQUE: fail | silent behavior change in parser", "CRITIQUE").expect("verdict");
    assert!(!passed);

    // Repair.
    let (word, _) = judges::parse_repair("REPAIR: applied | added the feature flag").expect("repair");
    assert_eq!(word, "applied");
    let (word, _) = judges::parse_repair("REPAIR: conflict | fix would break frozen contract").expect("repair");
    assert_eq!(word, "conflict");
    let (word, _) = judges::parse_repair("REPAIR: failed | action did not fix it").expect("repair");
    assert_eq!(word, "failed");
}

#[test]
fn next_walks_forward_and_stops_at_done() {
    use blackboard::Phase;
    use orchestrator::state_machine::next;
    assert_eq!(next(Phase::Preflight), Phase::Discovery);
    assert_eq!(next(Phase::Discovery), Phase::Contract);
    assert_eq!(next(Phase::Planning), Phase::Pilot);
    assert_eq!(next(Phase::FinalValidation), Phase::Done);
    assert_eq!(next(Phase::Done), Phase::Done);
}

#[test]
fn done_from_discovery_advances_not_self_transitions() {
    use blackboard::Phase;
    use orchestrator::state_machine;
    let mut state = state_at(Phase::Discovery);
    let next_phase = state_machine::next(state.phase);
    let record = state_machine::apply(&mut state, next_phase, "phase exit condition holds").expect("legal");
    assert_eq!(record.from, Phase::Discovery);
    assert_eq!(record.to, Phase::Contract);
}
