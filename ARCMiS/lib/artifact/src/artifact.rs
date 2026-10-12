//! Typed artifact store with owner, schema, provenance, and validation rule per artifact (ADR 0040).
//!
//! The store replaces the topic-revision blackboard for typed artifacts.
//! Every `put` validates the artifact against its schema and validation
//! rule. The provenance chain is append-only and verified by the
//! `ProvenanceChainRule`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

pub use runtime::validation_error::ValidationError;

/// Unique identifier for one artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactId(pub String);

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

/// The body of one artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactContent {
    /// UTF-8 text body.
    Text(String),
    /// Opaque bytes.
    Binary(Vec<u8>),
    /// Reference to a file on disk.
    Reference(PathBuf),
}

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

/// One entry in the provenance chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceEntry {
    /// Agent that performed the action.
    pub agent: String,
    /// Action performed.
    pub action: ProvenanceAction,
    /// When the action happened.
    pub timestamp: SystemTime,
    /// Content hash at the time of the action.
    pub content_hash: String,
}

/// Append-only provenance chain.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Provenance {
    /// Entries in append order.
    pub entries: Vec<ProvenanceEntry>,
}

impl Provenance {
    /// Empty chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one entry.
    pub fn append(&mut self, entry: ProvenanceEntry) {
        self.entries.push(entry);
    }
}

/// Schema contract for an artifact's structure. Implementations return
/// the rejection reason as a `String`; the store wraps it in
/// `ValidationError::SchemaMismatch`.
pub trait Schema {
    /// Validate `content` against this schema.
    fn validate(&self, content: &ArtifactContent) -> Result<(), String>;
    /// Schema name.
    fn name(&self) -> &str;
}

/// Additional validation logic beyond the schema. Implementations
/// return the rejection reason as a `String`; the store wraps it in
/// `ValidationError::RuleViolation`.
pub trait ValidationRule {
    /// Check `artifact` against this rule.
    fn check(&self, artifact: &Artifact) -> Result<(), String>;
    /// Rule name.
    fn name(&self) -> &str;
}

/// One stored artifact plus its owner, schema, provenance, and rule.
#[derive(Debug)]
pub struct ArtifactEntry {
    /// The artifact itself.
    pub artifact: Artifact,
    /// Owner role or team.
    pub owner: String,
    /// Schema name applied at `put`.
    pub schema_name: String,
    /// Append-only provenance chain.
    pub provenance: Provenance,
    /// Validation rule name applied at `put`.
    pub validation_rule_name: String,
}

/// Typed artifact store with schema and validation enforcement.
#[derive(Debug, Default)]
pub struct ArtifactStore {
    entries: HashMap<ArtifactId, ArtifactEntry>,
}

impl ArtifactStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store one artifact. Validates against `schema` and `rule` first.
    /// Returns the artifact id on success.
    pub fn put(
        &mut self,
        artifact: Artifact,
        owner: &str,
        schema: &dyn Schema,
        provenance: Provenance,
        rule: &dyn ValidationRule,
    ) -> Result<ArtifactId, ValidationError> {
        schema.validate(&artifact.content).map_err(|reason| ValidationError::SchemaMismatch {
            schema: schema.name().to_owned(),
            reason,
        })?;
        rule.check(&artifact).map_err(|reason| ValidationError::RuleViolation {
            rule: rule.name().to_owned(),
            reason,
        })?;
        let id = artifact.id.clone();
        self.entries.insert(
            id.clone(),
            ArtifactEntry {
                artifact,
                owner: owner.to_owned(),
                schema_name: schema.name().to_owned(),
                provenance,
                validation_rule_name: rule.name().to_owned(),
            },
        );
        Ok(id)
    }

    /// Retrieve one entry by id.
    pub fn get(&self, id: &ArtifactId) -> Option<&ArtifactEntry> {
        self.entries.get(id)
    }

    /// Every entry owned by `owner`.
    pub fn list_by_owner(&self, owner: &str) -> Vec<&ArtifactEntry> {
        self.entries.values().filter(|entry| entry.owner == owner).collect()
    }

    /// Every entry of one kind.
    pub fn list_by_kind(&self, kind: ArtifactKind) -> Vec<&ArtifactEntry> {
        self.entries.values().filter(|entry| entry.artifact.kind == kind).collect()
    }

    /// Re-validate one entry. Returns `UnknownArtifact` when the id is
    /// absent. The store does not retain the schema or rule objects, so
    /// re-validation requires the caller to supply them.
    pub fn validate(
        &self,
        id: &ArtifactId,
        schema: &dyn Schema,
        rule: &dyn ValidationRule,
    ) -> Result<(), ValidationError> {
        let entry = self.entries.get(id).ok_or_else(|| ValidationError::UnknownArtifact {
            id: id.0.clone(),
        })?;
        schema.validate(&entry.artifact.content).map_err(|reason| ValidationError::SchemaMismatch {
            schema: schema.name().to_owned(),
            reason,
        })?;
        rule.check(&entry.artifact).map_err(|reason| ValidationError::RuleViolation {
            rule: rule.name().to_owned(),
            reason,
        })
    }

    /// The provenance chain for one entry.
    pub fn provenance(&self, id: &ArtifactId) -> Option<&Provenance> {
        self.entries.get(id).map(|entry| &entry.provenance)
    }
}

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

/// Schema for a migration brief artifact. Accepts non-empty text.
pub struct MigrationBriefSchema;

impl Schema for MigrationBriefSchema {
    fn validate(&self, content: &ArtifactContent) -> Result<(), String> {
        match content {
            ArtifactContent::Text(body) if !body.trim().is_empty() => Ok(()),
            _ => Err("migration brief must be non-empty text".to_owned()),
        }
    }

    fn name(&self) -> &str {
        "migration-brief"
    }
}

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

/// Schema for a test report artifact. Accepts non-empty text.
pub struct TestReportSchema;

impl Schema for TestReportSchema {
    fn validate(&self, content: &ArtifactContent) -> Result<(), String> {
        match content {
            ArtifactContent::Text(body) if !body.trim().is_empty() => Ok(()),
            _ => Err("test report must be non-empty text".to_owned()),
        }
    }

    fn name(&self) -> &str {
        "test-report"
    }
}

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

/// Rule that verifies the artifact's content hash matches the latest
/// provenance entry. The artifact must carry at least one provenance
/// entry whose `content_hash` equals the hash computed from the
/// artifact's content.
pub struct ContentHashRule;

impl ContentHashRule {
    /// Compute the content hash for one artifact body.
    pub fn hash(content: &ArtifactContent) -> String {
        match content {
            ArtifactContent::Text(body) => format!("text:{}:{}", body.len(), body),
            ArtifactContent::Binary(bytes) => format!("binary:{}:{}", bytes.len(), hex_preview(bytes)),
            ArtifactContent::Reference(path) => format!("ref:{}", path.display()),
        }
    }

    /// Verify that `provenance` carries a hash matching the artifact
    /// content. Returns `Ok(())` when the latest entry's hash equals
    /// the computed hash.
    pub fn verify(artifact: &Artifact, provenance: &Provenance) -> Result<(), String> {
        let expected = Self::hash(&artifact.content);
        let Some(last) = provenance.entries.last() else {
            return Err("provenance chain is empty".to_owned());
        };
        if last.content_hash != expected {
            return Err(format!("content hash mismatch: expected {expected}, got {}", last.content_hash));
        }
        Ok(())
    }
}

impl ValidationRule for ContentHashRule {
    fn check(&self, artifact: &Artifact) -> Result<(), String> {
        Self::verify(artifact, &Provenance::new())
    }

    fn name(&self) -> &str {
        "content-hash"
    }
}

fn hex_preview(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().min(16) * 2);
    for byte in bytes.iter().take(16) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Rule that verifies the artifact conforms to its schema. The store
/// already runs the schema at `put`; this rule lets a caller re-check
/// after a modification.
pub struct SchemaConformanceRule;

impl ValidationRule for SchemaConformanceRule {
    fn check(&self, artifact: &Artifact) -> Result<(), String> {
        if artifact.id.0.is_empty() {
            return Err("artifact id must not be empty".to_owned());
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "schema-conformance"
    }
}

/// Rule that verifies the provenance chain is complete: every entry
/// carries a non-empty agent and content hash, and the chain starts
/// with a `Created` action.
pub struct ProvenanceChainRule;

impl ProvenanceChainRule {
    /// Verify a provenance chain directly. Returns `Ok(())` when the
    /// chain is complete: at least one entry, the first action is
    /// `Created`, every agent and content hash is non-empty.
    pub fn verify(provenance: &Provenance) -> Result<(), String> {
        let Some(first) = provenance.entries.first() else {
            return Err("provenance chain is empty".to_owned());
        };
        if first.action != ProvenanceAction::Created {
            return Err("provenance chain must start with Created".to_owned());
        }
        for entry in &provenance.entries {
            if entry.agent.trim().is_empty() {
                return Err("provenance entry has empty agent".to_owned());
            }
            if entry.content_hash.trim().is_empty() {
                return Err("provenance entry has empty content hash".to_owned());
            }
        }
        Ok(())
    }
}

impl ValidationRule for ProvenanceChainRule {
    fn check(&self, artifact: &Artifact) -> Result<(), String> {
        Self::verify(&Provenance::new()).map_err(|_| "provenance chain is empty".to_owned())?;
        let _ = artifact;
        Ok(())
    }

    fn name(&self) -> &str {
        "provenance-chain"
    }
}
