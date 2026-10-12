//! One round's fresh activity, for stagnation accounting.

/// One round's fresh activity, for stagnation accounting. Counts hold
/// round deltas, never cumulative totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoundOutcome {
    /// Work items dispatched this round.
    pub dispatched: usize,
    /// Work items completed this round.
    pub completed: usize,
}
