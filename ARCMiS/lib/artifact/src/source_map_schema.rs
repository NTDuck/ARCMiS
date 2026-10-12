//! Schema for a source map artifact.

use crate::artifact_content::ArtifactContent;
use crate::schema::Schema;

/// Schema for a source map artifact. Accepts non-empty text or a
/// reference to an existing path.
pub struct SourceMapSchema;

impl Schema for SourceMapSchema {
    fn validate(&self, content: &ArtifactContent) -> Result<(), String> {
        match content {
            ArtifactContent::Text(body) if !body.trim().is_empty() => Ok(()),
            ArtifactContent::Reference(path) if !path.as_os_str().is_empty() => Ok(()),
            _ => Err("source map must be non-empty text or a path".to_owned()),
        }
    }

    fn name(&self) -> &str {
        "source-map"
    }
}
