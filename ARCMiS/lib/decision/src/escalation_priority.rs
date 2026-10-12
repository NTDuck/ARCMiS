//! Escalation priority.

/// Escalation priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationPriority {
    /// Low priority: routine escalation.
    Low,
    /// Medium priority: standard escalation.
    Medium,
    /// High priority: urgent escalation.
    High,
}
