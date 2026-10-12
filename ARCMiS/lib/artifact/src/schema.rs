//! Schema contract for an artifact's structure.

use crate::artifact_content::ArtifactContent;

/// Schema contract for an artifact's structure. Implementations return
/// the rejection reason as a `String`; the store wraps it in
/// `ValidationError::SchemaMismatch`.
pub trait Schema {
    /// Validate `content` against this schema.
    fn validate(&self, content: &ArtifactContent) -> Result<(), String>;
    /// Schema name.
    fn name(&self) -> &str;
}
