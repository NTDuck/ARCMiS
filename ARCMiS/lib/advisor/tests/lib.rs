//! Integration tests for the advisor crate.

use mas_advisor::AdvisorConfig;
use mas_advisor::AdvisorError;
use mas_advisor::AdvisorVerdict;
use mas_advisor::OmpAdvisor;

#[test]
fn disabled_advisor_returns_defer() {
    let mut advisor = OmpAdvisor::new(AdvisorConfig::default());
    assert!(!advisor.is_enabled());
    let verdict = advisor.consult("question", "context").expect("disabled returns defer");
    assert_eq!(verdict, AdvisorVerdict::Defer);
}

#[test]
fn enabled_advisor_queues_consultation() {
    let mut advisor = OmpAdvisor::new(AdvisorConfig {
        enabled: true,
        ..AdvisorConfig::default()
    });
    assert!(advisor.is_enabled());
    let verdict = advisor.consult("question", "context").expect("consult queued");
    assert_eq!(verdict, AdvisorVerdict::Defer);
    let drained = advisor.drain_backlog();
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].question, "question");
}

#[test]
fn backlog_full_returns_error() {
    let mut advisor = OmpAdvisor::new(AdvisorConfig {
        enabled: true,
        sync_backlog: 1,
        ..AdvisorConfig::default()
    });
    advisor.consult("first", "context").expect("first queued");
    let error = advisor.consult("second", "context").expect_err("backlog full");
    assert!(matches!(error, AdvisorError::BacklogFull));
}
