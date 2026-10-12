//! Additional validation logic beyond the schema.

use crate::artifact::Artifact;

/// Additional validation logic beyond the schema. Implementations
/// return the rejection reason as a `String`; the store wraps it in
/// `ValidationError::RuleViolation`.
pub trait ValidationRule {
    /// Check `artifact` against this rule.
    fn check(&self, artifact: &Artifact) -> Result<(), String>;
    /// Rule name.
    fn name(&self) -> &str;
}
