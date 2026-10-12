//! One consultation the advisor received.
//!
//! The response is `None` while the consultation is pending and
//! `Some` once the advisor answers.

/// One consultation the advisor received. The response is `None`
/// while the consultation is pending and `Some` once the advisor
/// answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisorConsultation {
    /// The question the conductor asked.
    pub question: String,
    /// The context the conductor attached.
    pub context: String,
    /// The advisor's answer, when one arrives.
    pub response: Option<String>,
}
