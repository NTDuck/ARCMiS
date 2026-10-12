//! Ceilings and bounds from the run config's `limits` section.

/// Ceilings and bounds from the run config's `limits` section. Each one
/// bubbles to the call site: zero disables the bound it names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GraphBudgets {
    /// Maximum agents dispatched concurrently per round (ADR 0022).
    pub fanout: usize,
    /// Repairs per work item before escalation.
    pub max_repairs: usize,
    /// Consecutive stagnant rounds before the loop escalates.
    pub stagnation_rounds: usize,
}
