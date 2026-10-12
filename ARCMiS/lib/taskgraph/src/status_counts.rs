//! Node-count snapshot by status, for gate evidence and reporting.

/// Node-count snapshot by status, for gate evidence and reporting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StatusCounts {
    /// Nodes waiting on dependencies.
    pub queued: usize,
    /// Nodes holding a fan-out slot.
    pub admitted: usize,
    /// Nodes running.
    pub in_progress: usize,
    /// Nodes finished.
    pub done: usize,
    /// Nodes terminally failed.
    pub failed: usize,
}
