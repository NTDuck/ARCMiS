//! Artifact validation failure classes.
//!
//! Validation failure surfaced by the artifact store.

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
            ValidationError::SchemaMismatch {
                schema,
                reason,
            } => {
                write!(formatter, "schema {schema} rejected content: {reason}")
            },
            ValidationError::RuleViolation {
                rule,
                reason,
            } => {
                write!(formatter, "rule {rule} rejected artifact: {reason}")
            },
            ValidationError::UnknownArtifact {
                id,
            } => write!(formatter, "unknown artifact {id}"),
        }
    }
}

impl std::error::Error for ValidationError {}
