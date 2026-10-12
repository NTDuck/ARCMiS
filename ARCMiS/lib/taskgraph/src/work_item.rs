//! One unit of work.

use domain::Role;

use crate::kind::Kind;

/// One unit of work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItem {
    /// Specialist meant to pick the item up.
    pub role: Role,
    /// The node's typed workload.
    pub kind: Kind,
    /// Instruction text handed to the specialist.
    pub instruction: String,
}
