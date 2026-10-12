//! Retry strategy for one failure.

/// Retry strategy for one failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategy {
    /// Retry immediately.
    Immediate,
    /// Retry after a short delay.
    Delayed,
    /// Do not retry.
    None,
}
