//! Budgets and policy knobs for one run.

use std::time::Duration;

use taskgraph::GraphBudgets;

/// Budgets and policy knobs for one run. Every value bubbles from the
/// call site (config rule): the conductor never reads the environment.
#[derive(Debug, Clone)]
pub struct RunLimits {
    /// Graph budgets: fanout, repairs, stagnation.
    pub graph: GraphBudgets,
    /// Projected-token ceiling that triggers compaction.
    pub context_budget: usize,
    /// Per-event wait on the transport.
    pub event_timeout: Duration,
}

impl Default for RunLimits {
    fn default() -> Self {
        Self {
            graph: GraphBudgets::default(),
            context_budget: 100_000,
            event_timeout: Duration::from_secs(600),
        }
    }
}
