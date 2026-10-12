//! Integration tests for the test-outcomes crate.

use mas_test_outcomes::TestOutcomes;

#[test]
fn empty_outcomes_are_green() {
    let outcomes = TestOutcomes::new();
    assert!(outcomes.all_green());
    assert_eq!(outcomes.passed, 0);
    assert_eq!(outcomes.failed, 0);
    assert_eq!(outcomes.skipped, 0);
}

#[test]
fn recording_passes_keeps_green() {
    let mut outcomes = TestOutcomes::new();
    outcomes.record_pass();
    outcomes.record_pass();
    assert!(outcomes.all_green());
    assert_eq!(outcomes.passed, 2);
}

#[test]
fn recording_failure_marks_red() {
    let mut outcomes = TestOutcomes::new();
    outcomes.record_pass();
    outcomes.record_failure("test_a");
    assert!(!outcomes.all_green());
    assert_eq!(outcomes.failed, 1);
    assert_eq!(outcomes.failing_tests, vec!["test_a"]);
}

#[test]
fn recording_skip_does_not_affect_green() {
    let mut outcomes = TestOutcomes::new();
    outcomes.record_pass();
    outcomes.record_skip();
    assert!(outcomes.all_green());
    assert_eq!(outcomes.skipped, 1);
}
