//! Schema for a translated module artifact.

use crate::artifact_content::ArtifactContent;
use crate::schema::Schema;

/// Schema for a translated module artifact. Accepts non-empty text or
/// binary content.
pub struct TranslatedModuleSchema;

impl Schema for TranslatedModuleSchema {
    fn validate(&self, content: &ArtifactContent) -> Result<(), String> {
        match content {
            ArtifactContent::Text(body) if !body.trim().is_empty() => Ok(()),
            ArtifactContent::Binary(bytes) if !bytes.is_empty() => Ok(()),
            _ => Err("translated module must be non-empty text or binary".to_owned()),
        }
    }

    fn name(&self) -> &str {
        "translated-module"
    }
}
