//! Integration tests for the router module.

use domain::Role;
use mas_decision::DecisionSource;
use mas_decision::RouteBasis;
use mas_decision::Router;

#[test]
fn keyword_only_router_routes_by_keyword() {
    let router = Router::keyword_only();
    let decision = router.route_failure("borrow checker error");
    assert_eq!(decision.role, Role::UpliftWorker);
    assert!(matches!(decision.basis, RouteBasis::Keyword(DecisionSource::Keyword { .. })));
}

#[test]
fn keyword_only_router_falls_back_to_failure_analyst() {
    let router = Router::keyword_only();
    let decision = router.route_failure("something completely unknown");
    assert_eq!(decision.role, Role::FailureAnalyst);
    assert!(matches!(decision.basis, RouteBasis::Keyword(DecisionSource::KeywordMiss)));
}

#[test]
fn keyword_routing_maps_test_failures_to_tester() {
    let router = Router::keyword_only();
    let decision = router.route_failure("test failure: assertion failed");
    assert_eq!(decision.role, Role::DifferentialTester);
}

#[test]
fn keyword_routing_maps_linker_errors_to_failure_analyst() {
    let router = Router::keyword_only();
    let decision = router.route_failure("linker error: undefined reference");
    assert_eq!(decision.role, Role::FailureAnalyst);
}
