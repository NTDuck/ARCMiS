//! The kind of artifact the store holds.

/// The kind of artifact the store holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactKind {
    /// Source map produced by the discovery team.
    SourceMap,
    /// Migration brief produced by the planning team.
    MigrationBrief,
    /// Translated module produced by the migration team.
    TranslatedModule,
    /// Test report produced by the verification team.
    TestReport,
    /// Security audit produced by the security team.
    SecurityAudit,
    /// Business rule extracted from the source code.
    BusinessRule,
    /// Provenance record for an artifact.
    Provenance,
}
