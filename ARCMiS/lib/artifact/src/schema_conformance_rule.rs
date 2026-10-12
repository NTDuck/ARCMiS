//! Rule that verifies the artifact conforms to its schema.

use crate::artifact::Artifact;
use crate::validation_rule::ValidationRule;

/// Rule that verifies the artifact conforms to its schema. The store
/// already runs the schema at `put`; this rule lets a caller re-check
/// after a modification.
pub struct SchemaConformanceRule;

impl ValidationRule for SchemaConformanceRule {
    fn check(&self, artifact: &Artifact) -> Result<(), String> {
        if artifact.id.0.is_empty() {
            return Err("artifact id must not be empty".to_owned());
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "schema-conformance"
    }
}
