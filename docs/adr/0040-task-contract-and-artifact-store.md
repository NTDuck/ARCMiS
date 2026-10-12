# 0040. Task contract and artifact store

- **Date:** 2026-10-12
- **Status:** accepted
- **Extends:** [0031](0031-mas-core-conductor-over-omp-transport.md), [0036](0036-hierarchical-mas-claude-code-modernization.md)
- **Related:** [0037](0037-mas-core-refactor-implementation-plan.md)

## Context

The Claude Code Modernization workflow defines typed task contracts
with structured fields (task_id, phase, unit, strategy, risk,
required_role, required_inputs with hashes, write_scope,
max_attempts, verification_required) and an artifact store with
owner, schema, provenance, and validation rule per artifact. The
current `WorkItem` struct holds only `role`, `kind`, and
`instruction`. The current `Blackboard` holds topic revisions
without owner, schema, or validation. The refactor must grow both
surfaces while keeping the existing API byte-identical for
backward compatibility.

## Decision

### 1. TaskContract struct

The `WorkItem` struct grows into a `TaskContract` struct with
structured fields:

```rust
pub struct TaskContract {
    pub task_id: TaskId,
    pub phase: Phase,
    pub unit: Unit,
    pub strategy: Strategy,
    pub risk: Risk,
    pub required_role: Role,
    pub required_inputs: Vec<InputRef>,
    pub write_scope: WriteScope,
    pub max_attempts: usize,
    pub verification_required: bool,
}
```

Field definitions:

- `task_id`: unique identifier for the task. Generated at
  contract creation time. Format: `<phase>-<unit>-<index>`.
- `phase`: the run-level phase that owns the task.
- `unit`: the module or batch the task operates on. Format:
  `<module_path>` or `<batch_id>`.
- `strategy`: the migration strategy for the task. One of
  `Uplift`, `Transform`, `Reimagine`.
- `risk`: the risk level for the task. One of `Low`, `Medium`,
  `High`. Computed from the unit's complexity and the strategy's
  risk profile.
- `required_role`: the specialist role that must execute the
  task. Resolved by the router.
- `required_inputs`: the artifacts the task requires as input.
  Each input carries a content hash for provenance.
- `write_scope`: the paths the task may write. Enforced by the
  gateway.
- `max_attempts`: the maximum number of attempts before
  escalation. Default: 2.
- `verification_required`: whether the task requires
  verification before completion. Default: true.

### 2. TaskResult struct

A new `TaskResult` struct carries the outcome:

```rust
pub struct TaskResult {
    pub task_id: TaskId,
    pub verdict: Verdict,
    pub artifacts: Vec<ArtifactRef>,
    pub evidence: Evidence,
    pub attempts_used: usize,
    pub failure_class: Option<FailureClass>,
}
```

Field definitions:

- `task_id`: the task that produced the result.
- `verdict`: the verdict for the task. One of `Proven`,
  `PartlyProven`, `NotProven`.
- `artifacts`: the artifacts the task produced. Each artifact
  carries a reference to the artifact store entry.
- `evidence`: the evidence the task collected. Used for verdict
  computation.
- `attempts_used`: the number of attempts the task consumed.
- `failure_class`: the failure class if the task failed. One of
  `TranslationError`, `VerificationError`, `TimeoutError`,
  `BudgetExhausted`, `Escalated`.

### 3. ArtifactStore API

A new `ArtifactStore` struct replaces the `Blackboard` for typed
artifacts. The `Blackboard` stays as a thin wrapper for backward
compatibility.

```rust
pub struct ArtifactStore {
    entries: HashMap<ArtifactId, ArtifactEntry>,
}

pub struct ArtifactEntry {
    pub artifact: Artifact,
    pub owner: String,
    pub schema: SchemaRef,
    pub provenance: Provenance,
    pub validation_rule: ValidationRuleRef,
}

pub struct Artifact {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub content: ArtifactContent,
    pub created_at: SystemTime,
}

pub enum ArtifactContent {
    Text(String),
    Binary(Vec<u8>),
    Reference(PathBuf),
}
```

The `ArtifactStore` API:

- `put(artifact, owner, schema, provenance, validation_rule)`:
  store an artifact. Validates the artifact against its schema
  and validation rule. Returns the artifact id.
- `get(id)`: retrieve an artifact by id.
- `list_by_owner(owner)`: list artifacts owned by a role or
  team.
- `list_by_kind(kind)`: list artifacts of a given kind.
- `validate(id)`: re-validate an artifact against its schema
  and validation rule.
- `provenance(id)`: retrieve the provenance chain for an
  artifact.

### 4. Schema and validation

A `Schema` trait defines the contract for an artifact's
structure:

```rust
pub trait Schema {
    fn validate(&self, content: &ArtifactContent) -> Result<(), ValidationError>;
    fn name(&self) -> &str;
}
```

A `ValidationRule` trait defines additional validation logic:

```rust
pub trait ValidationRule {
    fn check(&self, artifact: &Artifact) -> Result<(), ValidationError>;
    fn name(&self) -> &str;
}
```

Built-in schemas:

- `SourceMapSchema`: validates a source map artifact.
- `MigrationBriefSchema`: validates a migration brief artifact.
- `TranslatedModuleSchema`: validates a translated module
  artifact.
- `TestReportSchema`: validates a test report artifact.
- `SecurityAuditSchema`: validates a security audit artifact.

Built-in validation rules:

- `ContentHashRule`: verifies the content hash matches the
  artifact content.
- `SchemaConformanceRule`: verifies the artifact conforms to
  its schema.
- `ProvenanceChainRule`: verifies the provenance chain is
  complete.

### 5. Provenance tracking

Every artifact carries a provenance chain. The chain records
every agent that touched the artifact:

```rust
pub struct Provenance {
    pub entries: Vec<ProvenanceEntry>,
}

pub struct ProvenanceEntry {
    pub agent: String,
    pub action: ProvenanceAction,
    pub timestamp: SystemTime,
    pub content_hash: ContentHash,
}

pub enum ProvenanceAction {
    Created,
    Modified,
    Validated,
    Cited,
}
```

The provenance chain is append-only. Every modification appends
a new entry. The chain is verified by the
`ProvenanceChainRule` validation rule.

### 6. InputRef and content hashes

Every `required_inputs` entry carries a content hash:

```rust
pub struct InputRef {
    pub artifact_id: ArtifactId,
    pub content_hash: ContentHash,
    pub purpose: String,
}
```

The content hash is computed at artifact creation time. The
`ContentHashRule` validation rule verifies the hash matches the
artifact content. A task that requires an input with a
mismatched hash fails with `ValidationError`.

### 7. WriteScope

The `write_scope` field names the paths the task may write:

```rust
pub struct WriteScope {
    pub allowed_paths: Vec<PathBuf>,
    pub denied_paths: Vec<PathBuf>,
}
```

The gateway enforces the write scope. A tool call that writes to
a path outside the allowed paths or inside the denied paths is
rejected with `ToolVerdict::Deny`.

### 8. Backward compatibility

The existing `WorkItem` struct stays. A `From<WorkItem> for
TaskContract` impl maps the old fields to the new fields:

- `role` maps to `required_role`.
- `kind` maps to `strategy` (via a lookup table).
- `instruction` maps to `unit` (via a parser).

The existing `Blackboard` struct stays. A `From<Blackboard> for
ArtifactStore` impl migrates topic revisions to artifact entries.
The existing topic-revision API continues to work through a thin
wrapper.

The existing `TaskGraph` struct stays. A `From<TaskGraph> for
TaskContract>` impl maps graph nodes to task contracts. The
existing `add`, `ready`, `admit`, `start`, `finish`,
`record_failure` methods continue to work.

## Consequences

- The `WorkItem` struct grows into `TaskContract`. Existing call
  sites that build a `WorkItem` from a `Kind` and an instruction
  continue to work through a `From` impl.
- The `Blackboard` grows into an `ArtifactStore`. Existing topic
  revision calls continue to work through a thin wrapper.
- The `TaskGraph` grows into a task contract graph. Existing
  graph operations continue to work.
- The artifact store validates every artifact against its schema
  and validation rule. A schema mismatch or validation failure
  returns a `ValidationError` that the conductor logs and treats
  as a task failure.
- The provenance chain records every agent that touched an
  artifact. The chain is append-only and verified by the
  `ProvenanceChainRule`.
- The content hash on every input reference prevents stale
  inputs. A task that requires an input with a mismatched hash
  fails with `ValidationError`.
- The write scope on every task contract prevents unauthorized
  writes. The gateway enforces the scope.

## Verification

- Unit: `TaskContract` builder produces a valid contract from
  the example config and rejects missing required fields.
- Unit: `TaskResult` builder produces a valid result from a
  task execution and rejects missing required fields.
- Unit: `ArtifactStore` accepts a typed artifact with owner,
  schema, provenance, and validation rule.
- Unit: `ArtifactStore` rejects an artifact that fails its
  schema validation.
- Unit: `ArtifactStore` rejects an artifact that fails its
  validation rule.
- Unit: `ProvenanceChainRule` verifies a complete provenance
  chain and rejects an incomplete chain.
- Unit: `ContentHashRule` verifies a matching content hash and
  rejects a mismatched hash.
- Unit: the gateway enforces the write scope and rejects a tool
  call that writes outside the allowed paths.
- Integration: the conductor runs the full phase walk over a
  scripted transport with the example config and produces
  artifacts with complete provenance chains.
