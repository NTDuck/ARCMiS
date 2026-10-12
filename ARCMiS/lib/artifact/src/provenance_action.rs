//! One action recorded in the provenance chain.

/// One action recorded in the provenance chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProvenanceAction {
    /// The agent created the artifact.
    Created,
    /// The agent modified the artifact.
    Modified,
    /// The agent validated the artifact.
    Validated,
    /// The agent cited the artifact.
    Cited,
}
