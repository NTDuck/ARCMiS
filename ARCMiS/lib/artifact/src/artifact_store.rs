//! Typed artifact store with schema and validation enforcement.

use std::collections::HashMap;

use runtime::validation_error::ValidationError;

use crate::artifact::Artifact;
use crate::artifact_entry::ArtifactEntry;
use crate::artifact_id::ArtifactId;
use crate::artifact_kind::ArtifactKind;
use crate::provenance::Provenance;
use crate::schema::Schema;
use crate::validation_rule::ValidationRule;

/// Typed artifact store with schema and validation enforcement.
#[derive(Debug, Default)]
pub struct ArtifactStore {
    entries: HashMap<ArtifactId, ArtifactEntry>,
}

impl ArtifactStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store one artifact. Validates against `schema` and `rule` first.
    /// Returns the artifact id on success.
    pub fn put(
        &mut self,
        artifact: Artifact,
        owner: &str,
        schema: &dyn Schema,
        provenance: Provenance,
        rule: &dyn ValidationRule,
    ) -> Result<ArtifactId, ValidationError> {
        schema.validate(&artifact.content).map_err(|reason| ValidationError::SchemaMismatch {
            schema: schema.name().to_owned(),
            reason,
        })?;
        rule.check(&artifact).map_err(|reason| ValidationError::RuleViolation {
            rule: rule.name().to_owned(),
            reason,
        })?;
        let id = artifact.id.clone();
        self.entries.insert(
            id.clone(),
            ArtifactEntry {
                artifact,
                owner: owner.to_owned(),
                schema_name: schema.name().to_owned(),
                provenance,
                validation_rule_name: rule.name().to_owned(),
            },
        );
        Ok(id)
    }

    /// Retrieve one entry by id.
    pub fn get(&self, id: &ArtifactId) -> Option<&ArtifactEntry> {
        self.entries.get(id)
    }

    /// Every entry owned by `owner`.
    pub fn list_by_owner(&self, owner: &str) -> Vec<&ArtifactEntry> {
        self.entries.values().filter(|entry| entry.owner == owner).collect()
    }

    /// Every entry of one kind.
    pub fn list_by_kind(&self, kind: ArtifactKind) -> Vec<&ArtifactEntry> {
        self.entries.values().filter(|entry| entry.artifact.kind == kind).collect()
    }

    /// Re-validate one entry. Returns `UnknownArtifact` when the id is
    /// absent. The store does not retain the schema or rule objects, so
    /// re-validation requires the caller to supply them.
    pub fn validate(
        &self,
        id: &ArtifactId,
        schema: &dyn Schema,
        rule: &dyn ValidationRule,
    ) -> Result<(), ValidationError> {
        let entry = self.entries.get(id).ok_or_else(|| ValidationError::UnknownArtifact {
            id: id.0.clone(),
        })?;
        schema.validate(&entry.artifact.content).map_err(|reason| ValidationError::SchemaMismatch {
            schema: schema.name().to_owned(),
            reason,
        })?;
        rule.check(&entry.artifact).map_err(|reason| ValidationError::RuleViolation {
            rule: rule.name().to_owned(),
            reason,
        })
    }

    /// The provenance chain for one entry.
    pub fn provenance(&self, id: &ArtifactId) -> Option<&Provenance> {
        self.entries.get(id).map(|entry| &entry.provenance)
    }
}
