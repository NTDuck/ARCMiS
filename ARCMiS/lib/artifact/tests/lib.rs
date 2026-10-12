//! Integration tests for the artifact crate.

use std::time::SystemTime;

use mas_artifact::Artifact;
use mas_artifact::ArtifactContent;
use mas_artifact::ArtifactId;
use mas_artifact::ArtifactKind;
use mas_artifact::ArtifactStore;
use mas_artifact::ContentHashRule;
use mas_artifact::MigrationBriefSchema;
use mas_artifact::Provenance;
use mas_artifact::ProvenanceAction;
use mas_artifact::ProvenanceChainRule;
use mas_artifact::ProvenanceEntry;
use mas_artifact::SchemaConformanceRule;
use mas_artifact::SecurityAuditSchema;
use mas_artifact::SourceMapSchema;
use mas_artifact::TestReportSchema;
use mas_artifact::TranslatedModuleSchema;
use mas_artifact::ValidationError;

fn text_artifact(id: &str, kind: ArtifactKind, body: &str) -> Artifact {
    Artifact {
        id: ArtifactId(id.to_owned()),
        kind,
        content: ArtifactContent::Text(body.to_owned()),
        created_at: SystemTime::now(),
    }
}

fn empty_provenance() -> Provenance {
    Provenance::new()
}

fn created_provenance(agent: &str, hash: &str) -> Provenance {
    let mut provenance = Provenance::new();
    provenance.append(ProvenanceEntry {
        agent: agent.to_owned(),
        action: ProvenanceAction::Created,
        timestamp: SystemTime::now(),
        content_hash: hash.to_owned(),
    });
    provenance
}

#[test]
fn put_accepts_a_valid_artifact() {
    let mut store = ArtifactStore::new();
    let artifact = text_artifact("a1", ArtifactKind::MigrationBrief, "brief body");
    let id = store
        .put(artifact, "strategy-selector", &MigrationBriefSchema, empty_provenance(), &SchemaConformanceRule)
        .expect("valid artifact accepted");
    assert_eq!(id, ArtifactId("a1".to_owned()));
    assert!(store.get(&id).is_some());
}

#[test]
fn put_rejects_artifact_failing_schema() {
    let mut store = ArtifactStore::new();
    let artifact = text_artifact("a1", ArtifactKind::MigrationBrief, "");
    let error = store
        .put(artifact, "strategy-selector", &MigrationBriefSchema, empty_provenance(), &SchemaConformanceRule)
        .expect_err("empty body rejected");
    assert!(matches!(error, ValidationError::SchemaMismatch { .. }));
}

#[test]
fn put_rejects_artifact_failing_validation_rule() {
    let mut store = ArtifactStore::new();
    let artifact = Artifact {
        id: ArtifactId(String::new()),
        kind: ArtifactKind::MigrationBrief,
        content: ArtifactContent::Text("body".to_owned()),
        created_at: SystemTime::now(),
    };
    let error = store
        .put(artifact, "strategy-selector", &MigrationBriefSchema, empty_provenance(), &SchemaConformanceRule)
        .expect_err("empty id rejected");
    assert!(matches!(error, ValidationError::RuleViolation { .. }));
}

#[test]
fn get_returns_the_stored_artifact() {
    let mut store = ArtifactStore::new();
    let artifact = text_artifact("a1", ArtifactKind::TestReport, "report body");
    let id = store
        .put(artifact, "differential-tester", &TestReportSchema, empty_provenance(), &SchemaConformanceRule)
        .expect("valid artifact accepted");
    let entry = store.get(&id).expect("entry present");
    assert_eq!(entry.artifact.id, id);
    assert_eq!(entry.owner, "differential-tester");
    assert_eq!(entry.schema_name, "test-report");
    assert_eq!(entry.validation_rule_name, "schema-conformance");
}

#[test]
fn list_by_owner_returns_artifacts_owned_by_a_role() {
    let mut store = ArtifactStore::new();
    let first = text_artifact("a1", ArtifactKind::SourceMap, "map one");
    let second = text_artifact("a2", ArtifactKind::MigrationBrief, "brief one");
    let third = text_artifact("a3", ArtifactKind::TestReport, "report one");
    store
        .put(first, "legacy-analyst", &SourceMapSchema, empty_provenance(), &SchemaConformanceRule)
        .expect("first accepted");
    store
        .put(second, "strategy-selector", &MigrationBriefSchema, empty_provenance(), &SchemaConformanceRule)
        .expect("second accepted");
    store
        .put(third, "strategy-selector", &TestReportSchema, empty_provenance(), &SchemaConformanceRule)
        .expect("third accepted");
    let planner = store.list_by_owner("strategy-selector");
    assert_eq!(planner.len(), 2);
    let analyst = store.list_by_owner("legacy-analyst");
    assert_eq!(analyst.len(), 1);
}

#[test]
fn list_by_kind_returns_artifacts_of_a_given_kind() {
    let mut store = ArtifactStore::new();
    let first = text_artifact("a1", ArtifactKind::SecurityAudit, "audit one");
    let second = text_artifact("a2", ArtifactKind::SecurityAudit, "audit two");
    let third = text_artifact("a3", ArtifactKind::TestReport, "report one");
    store
        .put(first, "security-auditor", &SecurityAuditSchema, empty_provenance(), &SchemaConformanceRule)
        .expect("first accepted");
    store
        .put(second, "security-auditor", &SecurityAuditSchema, empty_provenance(), &SchemaConformanceRule)
        .expect("second accepted");
    store
        .put(third, "differential-tester", &TestReportSchema, empty_provenance(), &SchemaConformanceRule)
        .expect("third accepted");
    let audits = store.list_by_kind(ArtifactKind::SecurityAudit);
    assert_eq!(audits.len(), 2);
    let reports = store.list_by_kind(ArtifactKind::TestReport);
    assert_eq!(reports.len(), 1);
}

#[test]
fn provenance_chain_rule_verifies_a_complete_chain() {
    let mut provenance = Provenance::new();
    provenance.append(ProvenanceEntry {
        agent: "analyst".to_owned(),
        action: ProvenanceAction::Created,
        timestamp: SystemTime::now(),
        content_hash: "hash-1".to_owned(),
    });
    provenance.append(ProvenanceEntry {
        agent: "reviewer".to_owned(),
        action: ProvenanceAction::Validated,
        timestamp: SystemTime::now(),
        content_hash: "hash-1".to_owned(),
    });
    assert!(ProvenanceChainRule::verify(&provenance).is_ok());
}

#[test]
fn provenance_chain_rule_rejects_an_empty_chain() {
    let provenance = Provenance::new();
    assert!(ProvenanceChainRule::verify(&provenance).is_err());
}

#[test]
fn provenance_chain_rule_rejects_a_chain_without_created() {
    let mut provenance = Provenance::new();
    provenance.append(ProvenanceEntry {
        agent: "reviewer".to_owned(),
        action: ProvenanceAction::Validated,
        timestamp: SystemTime::now(),
        content_hash: "hash-1".to_owned(),
    });
    assert!(ProvenanceChainRule::verify(&provenance).is_err());
}

#[test]
fn content_hash_rule_verifies_a_matching_hash() {
    let artifact = text_artifact("a1", ArtifactKind::TranslatedModule, "module body");
    let hash = ContentHashRule::hash(&artifact.content);
    let provenance = created_provenance("translator", &hash);
    assert!(ContentHashRule::verify(&artifact, &provenance).is_ok());
}

#[test]
fn content_hash_rule_rejects_a_mismatched_hash() {
    let artifact = text_artifact("a1", ArtifactKind::TranslatedModule, "module body");
    let provenance = created_provenance("translator", "wrong-hash");
    assert!(ContentHashRule::verify(&artifact, &provenance).is_err());
}

#[test]
fn translated_module_schema_accepts_binary_content() {
    let mut store = ArtifactStore::new();
    let artifact = Artifact {
        id: ArtifactId("a1".to_owned()),
        kind: ArtifactKind::TranslatedModule,
        content: ArtifactContent::Binary(vec![1, 2, 3]),
        created_at: SystemTime::now(),
    };
    store
        .put(artifact, "uplift-worker", &TranslatedModuleSchema, empty_provenance(), &SchemaConformanceRule)
        .expect("binary content accepted");
}
