//! Risk level for one task.

/// Risk level for one task. Computed from the unit's complexity and the
/// strategy's risk profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Risk {
    /// Low risk: routine translation.
    Low,
    /// Medium risk: structural change required.
    Medium,
    /// High risk: redesign or security-sensitive change.
    High,
}
