//! The advisor's answer to one consultation.

/// The advisor's answer to one consultation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvisorVerdict {
    /// The advisor answered the question.
    Answer(String),
    /// The advisor accepted the consultation but has no answer yet,
    /// or the advisor is disabled. The caller waits for the next
    /// drain or falls back to the next routing layer.
    Defer,
    /// The advisor returned an error string. The caller logs and
    /// falls back.
    Error(String),
}
