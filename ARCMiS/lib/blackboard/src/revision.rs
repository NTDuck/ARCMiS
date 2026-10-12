//! One published revision on one topic.

/// One published revision on one topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    /// Monotonic revision number within the topic.
    pub number: usize,
    /// Team or role that published.
    pub author: String,
    /// Body text.
    pub body: String,
}
