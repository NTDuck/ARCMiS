//! One stored artifact plus its owner, schema, provenance, and rule.

use crate::artifact::Artifact;
use crate::provenance::Provenance;

/// One stored artifact plus its owner, schema, provenance, and rule.
#[derive(Debug)]
pub struct ArtifactEntry {
    /// The artifact itself.
    pub artifact: Artifact,
    /// Owner role or team.
    pub owner: String,
    /// Schema name applied at `put`.
    pub schema_name: String,
    /// Append-only provenance chain.
    pub provenance: Provenance,
    /// Validation rule name applied at `put`.
    pub validation_rule_name: String,
}
