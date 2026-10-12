//! Integration tests for the jev module.

use mas_decision::EscalationPriority;
use mas_decision::FailureTriage;
use mas_decision::JevConfig;
use mas_decision::JevDecisionService;
use mas_decision::NextHandler;
use mas_decision::RetryStrategy;

#[test]
fn disabled_service_returns_none() {
    let service = JevDecisionService::new(JevConfig {
        enabled: false,
        ..JevConfig::default()
    });
    assert!(!service.is_enabled());
    assert!(service.triage("timeout").is_none());
}

#[test]
fn timeout_failure_is_retry() {
    let service = JevDecisionService::new(JevConfig::default());
    assert_eq!(service.triage("request timeout"), Some(FailureTriage::Retry));
}

#[test]
fn compile_failure_is_repair() {
    let service = JevDecisionService::new(JevConfig::default());
    assert_eq!(service.triage("compile error"), Some(FailureTriage::Repair));
}

#[test]
fn escalate_keyword_is_escalate() {
    let service = JevDecisionService::new(JevConfig::default());
    assert_eq!(service.triage("please escalate"), Some(FailureTriage::Escalate));
}

#[test]
fn triage_maps_to_next_handler() {
    let service = JevDecisionService::new(JevConfig::default());
    assert_eq!(service.next_handler(FailureTriage::Retry), NextHandler::Same);
    assert_eq!(service.next_handler(FailureTriage::Repair), NextHandler::Repair);
    assert_eq!(service.next_handler(FailureTriage::Escalate), NextHandler::Human);
}

#[test]
fn triage_maps_to_escalation_priority() {
    let service = JevDecisionService::new(JevConfig::default());
    assert_eq!(service.escalation_priority(FailureTriage::Retry), EscalationPriority::Low);
    assert_eq!(service.escalation_priority(FailureTriage::Repair), EscalationPriority::Medium);
    assert_eq!(service.escalation_priority(FailureTriage::Escalate), EscalationPriority::High);
}

#[test]
fn triage_maps_to_retry_strategy() {
    let service = JevDecisionService::new(JevConfig::default());
    assert_eq!(service.retry_strategy(FailureTriage::Retry), RetryStrategy::Immediate);
    assert_eq!(service.retry_strategy(FailureTriage::Repair), RetryStrategy::Delayed);
    assert_eq!(service.retry_strategy(FailureTriage::Escalate), RetryStrategy::None);
}
