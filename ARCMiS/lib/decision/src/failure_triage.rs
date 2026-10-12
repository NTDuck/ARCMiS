//! Failure triage decision.

/// Failure triage decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureTriage {
    /// Retry the same handler.
    Retry,
    /// Route to a repair handler.
    Repair,
    /// Escalate to a human.
    Escalate,
}
