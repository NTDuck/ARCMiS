//! Schema for a security audit artifact.

use crate::artifact_content::ArtifactContent;
use crate::schema::Schema;

/// Schema for a security audit artifact. Accepts non-empty text.
pub struct SecurityAuditSchema;

impl Schema for SecurityAuditSchema {
    fn validate(&self, content: &ArtifactContent) -> Result<(), String> {
        match content {
            ArtifactContent::Text(body) if !body.trim().is_empty() => Ok(()),
            _ => Err("security audit must be non-empty text".to_owned()),
        }
    }

    fn name(&self) -> &str {
        "security-audit"
    }
}
