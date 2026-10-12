//! Result of one compaction pass.

use crate::context_frame::ContextFrame;

/// Result of one compaction pass.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CompactionOutcome {
    /// Frames retained, in original order.
    pub kept: Vec<ContextFrame>,
    /// Frames archived, oldest first.
    pub archived: Vec<ContextFrame>,
}
