//! Gateway verdict.

/// Gateway verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolVerdict {
    /// The call may proceed.
    Allow,
    /// The gateway refused the call. Carries the matched pattern.
    Deny {
        /// Pattern that matched the call.
        matched: String,
    },
}
