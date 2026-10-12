//! Outcome of one task execution.

use artifact::Verdict;
use domain::Evidence;

use crate::failure_class::FailureClass;
use crate::task_id::TaskId;

/// Outcome of one task execution.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskResult {
    /// Task that produced the result.
    pub task_id: TaskId,
    /// Verdict for the task.
    pub verdict: Verdict,
    /// Artifacts the task produced.
    pub artifacts: Vec<String>,
    /// Evidence the task collected.
    pub evidence: Evidence,
    /// Attempts the task consumed.
    pub attempts_used: usize,
    /// Failure class when the task failed.
    pub failure_class: Option<FailureClass>,
}
