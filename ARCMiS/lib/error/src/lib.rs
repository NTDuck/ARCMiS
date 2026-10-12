//! `mas_error` names the run failure classes callers must distinguish.
//!
//! Every variant names a distinct recovery posture: restart the
//! transport, correct the configuration, tighten the gateway policy,
//! or raise the budgets. The error types live together so the
//! conductor and its collaborators share one vocabulary.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

/// One conductor run failure. Every variant names a distinct recovery
/// posture: restart the transport, correct the configuration, tighten
/// the gateway policy, or raise the budgets.
#[derive(Debug, thiserror::Error)]
pub enum ConductorError {
    /// The omp transport closed or failed its handshake. Restart the
    /// session and re-enter the run loop.
    #[error("transport failed: {0}")]
    Transport(String),
    /// The run configuration was invalid. Correct the file and start a new
    /// run: no retry corrects a bad configuration.
    #[error("configuration invalid: {0}")]
    Config(String),
    /// The tool gateway denied a call pattern that the run marked
    /// mandatory. Tighten the call pattern or loosen the policy.
    #[error("gateway rejected a mandatory call: {0}")]
    Gateway(String),
    /// The transport closed while work remained outstanding.
    #[error("budget exhausted: {0}")]
    BudgetExhausted(String),
    /// The run regressed until a regression edge held no rework path.
    #[error("regression exhausted: {0}")]
    RegressionExhausted(String),
}

/// Errors a transport may surface, decoupled from the SDK internals.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// The child closed or died.
    #[error("transport closed")]
    Closed,
    /// A request could not be written.
    #[error("send failed: {0}")]
    Send(String),
    /// A wait timed out.
    #[error("recv timed out")]
    Timeout,
    /// The transport refused the payload shape.
    #[error("bad request: {0}")]
    BadRequest(String),
}

/// Why a consultation did not produce an answer.
#[derive(Debug, thiserror::Error)]
pub enum AdvisorError {
    /// The advisor is disabled in the run config.
    #[error("advisor disabled")]
    Disabled,
    /// The transport refused the consultation.
    #[error("transport error: {0}")]
    TransportError(#[from] TransportError),
    /// The backlog is full. The caller should defer.
    #[error("advisor backlog full")]
    BacklogFull,
}

/// Validation failure surfaced by the artifact store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// The content did not match the schema.
    SchemaMismatch {
        /// Schema that rejected the content.
        schema: String,
        /// Why the schema rejected the content.
        reason: String,
    },
    /// A validation rule rejected the artifact.
    RuleViolation {
        /// Rule that rejected the artifact.
        rule: String,
        /// Why the rule rejected the artifact.
        reason: String,
    },
    /// The store has no artifact with this id.
    UnknownArtifact {
        /// The unknown id.
        id: String,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::SchemaMismatch { schema, reason } => {
                write!(formatter, "schema {schema} rejected content: {reason}")
            },
            ValidationError::RuleViolation { rule, reason } => {
                write!(formatter, "rule {rule} rejected artifact: {reason}")
            },
            ValidationError::UnknownArtifact { id } => write!(formatter, "unknown artifact {id}"),
        }
    }
}

impl std::error::Error for ValidationError {}
