//! One context frame awaiting a keep-or-archive decision.

/// One context frame awaiting a keep-or-archive decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextFrame {
    /// Frame text.
    pub text: String,
    /// Who produced the frame. Used in the archive stub.
    pub source: String,
}
