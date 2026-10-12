//! Integration tests for the error types.

use mas_runtime::AdvisorError;
use mas_runtime::ConductorError;
use mas_runtime::TransportError;
use mas_runtime::ValidationError;

#[test]
fn conductor_error_display_names_the_recovery_posture() {
    let transport = ConductorError::Transport("closed".to_owned());
    assert_eq!(transport.to_string(), "transport failed: closed");
    let config = ConductorError::Config("missing teams".to_owned());
    assert_eq!(config.to_string(), "configuration invalid: missing teams");
    let gateway = ConductorError::Gateway("denied".to_owned());
    assert_eq!(gateway.to_string(), "gateway rejected a mandatory call: denied");
    let budget = ConductorError::BudgetExhausted("fanout".to_owned());
    assert_eq!(budget.to_string(), "budget exhausted: fanout");
    let regression = ConductorError::RegressionExhausted("no path".to_owned());
    assert_eq!(regression.to_string(), "regression exhausted: no path");
}

#[test]
fn transport_error_display_names_the_failure_class() {
    assert_eq!(TransportError::Closed.to_string(), "transport closed");
    assert_eq!(TransportError::Send("io".to_owned()).to_string(), "send failed: io");
    assert_eq!(TransportError::Timeout.to_string(), "recv timed out");
    assert_eq!(TransportError::BadRequest("shape".to_owned()).to_string(), "bad request: shape");
}

#[test]
fn advisor_error_wraps_transport_error() {
    let transport = TransportError::Closed;
    let wrapped: AdvisorError = transport.into();
    assert!(matches!(wrapped, AdvisorError::TransportError(TransportError::Closed)));
    assert_eq!(AdvisorError::Disabled.to_string(), "advisor disabled");
    assert_eq!(AdvisorError::BacklogFull.to_string(), "advisor backlog full");
}

#[test]
fn validation_error_display_names_the_rejection() {
    let schema = ValidationError::SchemaMismatch {
        schema: "source-map".to_owned(),
        reason: "empty".to_owned(),
    };
    assert_eq!(schema.to_string(), "schema source-map rejected content: empty");
    let rule = ValidationError::RuleViolation {
        rule: "content-hash".to_owned(),
        reason: "mismatch".to_owned(),
    };
    assert_eq!(rule.to_string(), "rule content-hash rejected artifact: mismatch");
    let unknown = ValidationError::UnknownArtifact {
        id: "a1".to_owned(),
    };
    assert_eq!(unknown.to_string(), "unknown artifact a1");
}

#[test]
fn validation_error_equality_holds() {
    let first = ValidationError::SchemaMismatch {
        schema: "s".to_owned(),
        reason: "r".to_owned(),
    };
    let second = ValidationError::SchemaMismatch {
        schema: "s".to_owned(),
        reason: "r".to_owned(),
    };
    assert_eq!(first, second);
}
