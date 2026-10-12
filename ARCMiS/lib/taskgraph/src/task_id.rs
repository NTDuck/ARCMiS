//! Unique identifier for one task contract.

/// Unique identifier for one task contract. Format: `<phase>-<unit>-<index>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(pub String);
