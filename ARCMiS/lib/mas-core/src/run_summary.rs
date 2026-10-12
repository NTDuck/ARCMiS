//! End-of-run summary.

use artifact::Verdict;
use domain::Phase;

/// End-of-run summary.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSummary {
    /// Final phase the machine reached.
    pub final_phase: Phase,
    /// How the run ended.
    pub verdict: Verdict,
    /// Completed dispatch count.
    pub dispatches: usize,
    /// Compaction passes over the run.
    pub compactions: usize,
}
