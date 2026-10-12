//! Reference to one required input artifact.

/// Reference to one required input artifact. Carries the content hash
/// for provenance verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputRef {
    /// Artifact id in the store.
    pub artifact_id: String,
    /// Content hash at the time the input was bound.
    pub content_hash: String,
    /// Why the task needs this input.
    pub purpose: String,
}
