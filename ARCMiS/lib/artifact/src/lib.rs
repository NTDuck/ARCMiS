//! `mas_artifact` is the artifact chain for the MAS conductor.
//!
//! It bundles the typed artifact store with owner, schema, provenance,
//! and validation rule per artifact, plus the verdict computation that
//! rolls per-task evidence into one run-level verdict.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod artifact;
pub mod artifact_content;
pub mod artifact_entry;
pub mod artifact_id;
pub mod artifact_kind;
pub mod artifact_store;
pub mod content_hash_rule;
pub mod migration_brief_schema;
pub mod provenance;
pub mod provenance_action;
pub mod provenance_chain_rule;
pub mod provenance_entry;
pub mod schema;
pub mod schema_conformance_rule;
pub mod security_audit_schema;
pub mod source_map_schema;
pub mod test_report_schema;
pub mod translated_module_schema;
pub mod validation_rule;
pub mod verdict;

pub use artifact::Artifact;
pub use artifact_content::ArtifactContent;
pub use artifact_entry::ArtifactEntry;
pub use artifact_id::ArtifactId;
pub use artifact_kind::ArtifactKind;
pub use artifact_store::ArtifactStore;
pub use content_hash_rule::ContentHashRule;
pub use migration_brief_schema::MigrationBriefSchema;
pub use provenance::Provenance;
pub use provenance_action::ProvenanceAction;
pub use provenance_chain_rule::ProvenanceChainRule;
pub use provenance_entry::ProvenanceEntry;
pub use schema::Schema;
pub use schema_conformance_rule::SchemaConformanceRule;
pub use security_audit_schema::SecurityAuditSchema;
pub use source_map_schema::SourceMapSchema;
pub use test_report_schema::TestReportSchema;
pub use translated_module_schema::TranslatedModuleSchema;
pub use validation_rule::ValidationRule;
pub use verdict::{TaskEvidence, Verdict, VerdictComputer};
pub use runtime::validation_error::ValidationError;
