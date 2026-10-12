//! Typed task contract (ADR 0040 section 1).

use domain::Phase;
use domain::Role;

use crate::input_ref::InputRef;
use crate::risk::Risk;
use crate::strategy::Strategy;
use crate::task_id::TaskId;
use crate::write_scope::WriteScope;

/// Typed task contract (ADR 0040 section 1). The `WorkItem` grows into
/// this struct while the old fields stay for backward compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskContract {
    /// Unique identifier.
    pub task_id: TaskId,
    /// Run-level phase that owns the task.
    pub phase: Phase,
    /// Module or batch the task operates on.
    pub unit: String,
    /// Migration strategy.
    pub strategy: Strategy,
    /// Risk level.
    pub risk: Risk,
    /// Specialist role that must execute the task.
    pub required_role: Role,
    /// Required input artifacts with content hashes.
    pub required_inputs: Vec<InputRef>,
    /// Paths the task may write.
    pub write_scope: WriteScope,
    /// Maximum attempts before escalation.
    pub max_attempts: usize,
    /// Whether the task requires verification before completion.
    pub verification_required: bool,
}

impl TaskContract {
    /// Default maximum attempts before escalation.
    pub const DEFAULT_MAX_ATTEMPTS: usize = 2;
}
