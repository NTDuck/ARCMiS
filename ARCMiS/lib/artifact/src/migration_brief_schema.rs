//! Schema for a migration brief artifact.

use crate::artifact_content::ArtifactContent;
use crate::schema::Schema;

/// Schema for a migration brief artifact. Accepts non-empty text.
pub struct MigrationBriefSchema;

impl Schema for MigrationBriefSchema {
    fn validate(&self, content: &ArtifactContent) -> Result<(), String> {
        match content {
            ArtifactContent::Text(body) if !body.trim().is_empty() => Ok(()),
            _ => Err("migration brief must be non-empty text".to_owned()),
        }
    }

    fn name(&self) -> &str {
        "migration-brief"
    }
}
