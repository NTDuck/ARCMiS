//! Status of one work item.

/// Status of one work item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// Waiting for dependencies.
    Queued,
    /// Accepted under the fan-out bound, waiting for admission.
    Admitted,
    /// Running.
    InProgress,
    /// Finished successfully.
    Done,
    /// Terminally failed: budget spent on repairs.
    Failed,
}
