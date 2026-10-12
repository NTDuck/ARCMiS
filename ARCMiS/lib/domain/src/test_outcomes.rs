//! Test outcome aggregation for the MAS conductor.
//!
//! The conductor collects per-module test outcomes and rolls them up
//! into one `TestOutcomes` record. The record carries the pass, fail,
//! and skip counts plus the names of failing tests for triage.

/// Aggregated test outcomes for one run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestOutcomes {
    /// Number of tests that passed.
    pub passed: usize,
    /// Number of tests that failed.
    pub failed: usize,
    /// Number of tests that were skipped.
    pub skipped: usize,
    /// Names of failing tests, in collection order.
    pub failing_tests: Vec<String>,
}

impl TestOutcomes {
    /// Empty outcome record.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one passing test.
    pub fn record_pass(&mut self) {
        self.passed += 1;
    }

    /// Record one failing test.
    pub fn record_failure(&mut self, name: &str) {
        self.failed += 1;
        self.failing_tests.push(name.to_owned());
    }

    /// Record one skipped test.
    pub fn record_skip(&mut self) {
        self.skipped += 1;
    }

    /// Whether every recorded test passed.
    pub fn all_green(&self) -> bool {
        self.failed == 0
    }
}
