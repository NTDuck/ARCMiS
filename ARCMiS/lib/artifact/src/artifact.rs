//! One artifact stored in the store.

use std::time::SystemTime;

use crate::artifact_content::ArtifactContent;
use crate::artifact_id::ArtifactId;
use crate::artifact_kind::ArtifactKind;

/// One artifact stored in the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    /// Unique identifier.
    pub id: ArtifactId,
    /// Kind of artifact.
    pub kind: ArtifactKind,
    /// Body content.
    pub content: ArtifactContent,
    /// Creation timestamp.
    pub created_at: SystemTime,
}
