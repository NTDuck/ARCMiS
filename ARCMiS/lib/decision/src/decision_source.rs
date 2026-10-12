//! How a decision was reached.
//!
//! Every variant names its basis.

/// How a decision was reached. Every variant names its basis.
#[derive(Debug, Clone, PartialEq)]
pub enum DecisionSource {
    /// A laya choice answer picked a verdict label.
    Triage {
        /// Verdict label from the triage agent.
        verdict: String,
        /// Confidence score from the triage agent.
        confidence: f64,
    },
    /// No keyword matched. Deterministic fallback.
    KeywordMiss,
    /// A keyword rule matched.
    Keyword {
        /// Keyword that matched.
        keyword: &'static str,
    },
}
