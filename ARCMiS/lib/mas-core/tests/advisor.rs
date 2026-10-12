//! Integration tests for the OMP advisor handle (ADR 0036 section 6).

use mas_core::advisor::AdvisorConfig;
use mas_core::advisor::AdvisorError;
use mas_core::advisor::AdvisorVerdict;
use mas_core::advisor::OmpAdvisor;
use mas_core::transport::OmpLaunchConfig;

fn disabled_config() -> AdvisorConfig {
    AdvisorConfig {
        enabled: false,
        sync_backlog: 16,
        launch: OmpLaunchConfig {
            read_only: true,
            ..OmpLaunchConfig::default()
        },
    }
}

fn enabled_config(backlog: usize) -> AdvisorConfig {
    AdvisorConfig {
        enabled: true,
        sync_backlog: backlog,
        launch: OmpLaunchConfig {
            read_only: true,
            ..OmpLaunchConfig::default()
        },
    }
}

#[test]
fn disabled_advisor_returns_defer() {
    let mut advisor = OmpAdvisor::new(disabled_config());
    assert!(!advisor.is_enabled());
    let verdict = advisor.consult("what is the verdict?", "context").expect("disabled consult returns Defer");
    assert_eq!(verdict, AdvisorVerdict::Defer);
    assert!(advisor.drain_backlog().is_empty());
}

#[test]
fn enabled_advisor_with_empty_backlog_accepts_consultations() {
    let mut advisor = OmpAdvisor::new(enabled_config(16));
    assert!(advisor.is_enabled());
    let verdict = advisor.consult("classify this", "context").expect("empty backlog accepts the consult");
    assert_eq!(verdict, AdvisorVerdict::Defer);
    let pending = advisor.drain_backlog();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].question, "classify this");
    assert_eq!(pending[0].context, "context");
    assert_eq!(pending[0].response, None);
}

#[test]
fn full_backlog_returns_backlog_full() {
    let mut advisor = OmpAdvisor::new(enabled_config(2));
    advisor.consult("one", "ctx").expect("first consult accepted");
    advisor.consult("two", "ctx").expect("second consult accepted");
    let error = advisor.consult("three", "ctx").expect_err("third consult rejected");
    assert!(matches!(error, AdvisorError::BacklogFull));
    let pending = advisor.drain_backlog();
    assert_eq!(pending.len(), 2);
}

#[test]
fn drain_backlog_returns_all_pending_consultations() {
    let mut advisor = OmpAdvisor::new(enabled_config(16));
    advisor.consult("first", "ctx-a").expect("first consult accepted");
    advisor.consult("second", "ctx-b").expect("second consult accepted");
    advisor.consult("third", "ctx-c").expect("third consult accepted");
    let pending = advisor.drain_backlog();
    assert_eq!(pending.len(), 3);
    assert_eq!(pending[0].question, "first");
    assert_eq!(pending[0].context, "ctx-a");
    assert_eq!(pending[1].question, "second");
    assert_eq!(pending[1].context, "ctx-b");
    assert_eq!(pending[2].question, "third");
    assert_eq!(pending[2].context, "ctx-c");
    assert!(advisor.drain_backlog().is_empty());
}

#[test]
fn default_config_is_disabled_with_read_only_launch() {
    let config = AdvisorConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.sync_backlog, 16);
    assert!(config.launch.read_only);
}
