//! One point-to-point message.

/// One point-to-point message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Sender address.
    pub from: String,
    /// Body text.
    pub body: String,
}
