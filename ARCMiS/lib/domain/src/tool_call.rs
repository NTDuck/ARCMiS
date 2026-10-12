//! One tool call awaiting a verdict.

/// One tool call awaiting a verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    /// Tool name, e.g. `bash` or `edit`.
    pub tool: String,
    /// Raw argument text.
    pub args: String,
}
