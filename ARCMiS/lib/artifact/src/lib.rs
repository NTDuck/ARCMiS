//! `mas_artifact` is the artifact chain for the MAS conductor.
//!
//! It bundles the typed artifact store with owner, schema, provenance,
//! and validation rule per artifact, plus the verdict computation that
//! rolls per-task evidence into one run-level verdict.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod artifact;
pub mod verdict;

pub use artifact::{
    Artifact, ArtifactContent, ArtifactEntry, ArtifactId, ArtifactKind, ArtifactStore, ContentHashRule,
    MigrationBriefSchema, Provenance, ProvenanceAction, ProvenanceChainRule, ProvenanceEntry, Schema,
    SchemaConformanceRule, SecurityAuditSchema, SourceMapSchema, TestReportSchema, TranslatedModuleSchema,
    ValidationError, ValidationRule,
};
pub use verdict::{TaskEvidence, Verdict, VerdictComputer};
