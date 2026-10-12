//! Paths the task may write.

use std::path::PathBuf;

/// Paths the task may write. The gateway enforces the scope.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteScope {
    /// Paths the task may write.
    pub allowed_paths: Vec<PathBuf>,
    /// Paths the task may never write, even when inside an allowed path.
    pub denied_paths: Vec<PathBuf>,
}
