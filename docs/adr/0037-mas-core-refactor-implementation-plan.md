# 0037. mas-core refactor implementation plan

- **Date:** 2026-10-12
- **Status:** accepted
- **Extends:** [0036](0036-hierarchical-mas-claude-code-modernization.md)
- **Related:** [0038](0038-role-registry-claude-code-modernization.md), [0039](0039-two-level-phase-machine.md), [0040](0040-task-contract-and-artifact-store.md)

## Context

ADR 0036 defines the target shape of the refactored `mas-core`
conductor. This ADR orders the changes, names the files, and maps
the dependency graph between them. The plan follows the repo's
commit discipline: one verified change per commit, no batching of
unrelated work, every commit green before the next lands.

## Decision

The refactor lands in nine ordered phases. Each phase ends with a
green tree (`cargo fmt --check`, `cargo clippy --workspace
--all-targets`, `cargo nextest run --profile ci --workspace
--all-targets --no-tests=pass`). Each phase produces one or more
commits. The phases are sequential: phase N depends on phase N-1.

### Phase 1: Role registry extension (ADR 0038)

**Files:**
- `ARCMiS/lib/mas-core/src/hierarchy.rs` — add 12 new `Role`
  variants, update `ALL`, `name`, `from_name`.
- `ARCMiS/lib/run-config/src/hierarchy.rs` — add `strategy` field
  to `TeamConfig` (optional, default `null`).
- `ARCMiS/lib/run-config/src/validation.rs` — accept the new
  member names.
- `ARCMiS/lib/mas-core/tests/hierarchy.rs` — add tests for the new
  roles and the six-team compilation.

**Acceptance:** `cargo nextest run --profile ci -p ARCMiS-mas-core
--no-tests=pass` passes. The example config compiles to a
`TeamRegistry` with six teams.

### Phase 2: Phase machine extension (ADR 0039)

**Files:**
- `ARCMiS/lib/mas-core/src/phase.rs` — add 4 new `Phase` variants
  (`Assess`, `ExtractRules`, `ReviewRules`, `DraftBrief`,
  `MigratePilot`, `VerifyPilot`, `VerifyModule`, `FinalVerify`,
  `Harden`, `HumanSignOff`), update `ORDER`, `forward_next`,
  `regression_target`, `is_legal`.
- `ARCMiS/lib/mas-core/src/module_phase.rs` — new file. Define
  `ModulePhase` enum, `ModuleStateMachine` struct, evidence gates,
  failure transitions.
- `ARCMiS/lib/mas-core/src/lib.rs` — add `pub mod module_phase;`.
- `ARCMiS/lib/mas-core/tests/phase.rs` — add tests for the new
  phases and the module state machine.

**Acceptance:** `cargo nextest run --profile ci -p ARCMiS-mas-core
--no-tests=pass` passes. The phase machine walks the full forward
path from `Init` to `Done`.

### Phase 3: Task contract and artifact store (ADR 0040)

**Files:**
- `ARCMiS/lib/mas-core/src/taskgraph.rs` — add `TaskContract`
  struct, `TaskResult` struct, `From<WorkItem> for TaskContract`
  impl.
- `ARCMiS/lib/mas-core/src/artifact.rs` — new file. Define
  `Artifact` struct, `ArtifactStore` struct, `Schema` trait,
  `ValidationRule` trait, provenance tracking.
- `ARCMiS/lib/mas-core/src/blackboard.rs` — add `ArtifactStore`
  wrapper, keep existing topic-revision API.
- `ARCMiS/lib/mas-core/src/lib.rs` — add `pub mod artifact;`.
- `ARCMiS/lib/mas-core/tests/taskgraph.rs` — add tests for
  `TaskContract` and `TaskResult`.
- `ARCMiS/lib/mas-core/tests/artifact.rs` — new file. Tests for
  `ArtifactStore`.

**Acceptance:** `cargo nextest run --profile ci -p ARCMiS-mas-core
--no-tests=pass` passes. The artifact store accepts a typed
artifact and rejects one that fails its validation rule.

### Phase 4: Adaptive router

**Files:**
- `ARCMiS/lib/mas-core/src/router.rs` — add four-layer cascade:
  deterministic signals, repository metadata, test outcomes,
  bounded model judgments. Add `RouteSignal` enum, `RouteContext`
  struct.
- `ARCMiS/lib/mas-core/src/repository.rs` — new file. Define
  `RepositoryMetadata` struct (file topology, dependency graph,
  test coverage).
- `ARCMiS/lib/mas-core/src/test_outcomes.rs` — new file. Define
  `TestOutcomes` struct (pass/fail counts, coverage deltas,
  regression markers).
- `ARCMiS/lib/mas-core/src/lib.rs` — add `pub mod repository;`,
  `pub mod test_outcomes;`.
- `ARCMiS/lib/mas-core/tests/router.rs` — new file. Tests for the
  four-layer cascade.

**Acceptance:** `cargo nextest run --profile ci -p ARCMiS-mas-core
--no-tests=pass` passes. The router returns a `RouteDecision` for
every signal combination.

### Phase 5: Verdicts

**Files:**
- `ARCMiS/lib/mas-core/src/verdict.rs` — new file. Define
  `Verdict` enum (`Proven`, `PartlyProven`, `NotProven`),
  `VerdictComputer` function.
- `ARCMiS/lib/mas-core/src/conductor.rs` — replace `Verdict` with
  re-export from `verdict`, add `compute_verdict` method.
- `ARCMiS/lib/mas-core/src/lib.rs` — add `pub mod verdict;`.
- `ARCMiS/lib/mas-core/tests/verdict.rs` — new file. Tests for
  `VerdictComputer`.

**Acceptance:** `cargo nextest run --profile ci -p ARCMiS-mas-core
--no-tests=pass` passes. The verdict computer returns the correct
verdict for every evidence combination.

### Phase 6: OMP advisor integration

**Files:**
- `ARCMiS/lib/mas-core/src/advisor.rs` — new file. Define
  `OmpAdvisor` struct, `AdvisorConfig` struct, `syncBacklog` field.
- `ARCMiS/lib/mas-core/src/transport.rs` — add `read_only` flag to
  `OmpLaunchConfig`.
- `ARCMiS/lib/mas-core/src/conductor.rs` — add `advisor` field,
  `consult_advisor` method.
- `ARCMiS/lib/mas-core/src/lib.rs` — add `pub mod advisor;`.
- `ARCMiS/lib/mas-core/tests/advisor.rs` — new file. Tests for
  `OmpAdvisor`.

**Acceptance:** `cargo nextest run --profile ci -p ARCMiS-mas-core
--no-tests=pass` passes. The advisor accepts a read-only
consultation and rejects a write consultation.

### Phase 7: JEV decision model service

**Files:**
- `ARCMiS/lib/mas-core/src/jev.rs` — new file. Define
  `JevDecisionService` struct, four decision classes
  (`FailureTriage`, `NextHandler`, `EscalationPriority`,
  `RetryStrategy`), trained workflow mapping.
- `ARCMiS/lib/mas-core/src/router.rs` — replace single-question
  triage with `JevDecisionService`.
- `ARCMiS/lib/mas-core/src/lib.rs` — add `pub mod jev;`.
- `ARCMiS/lib/mas-core/tests/jev.rs` — new file. Tests for
  `JevDecisionService`.

**Acceptance:** `cargo nextest run --profile ci -p ARCMiS-mas-core
--no-tests=pass` passes. The JEV service answers all four decision
classes and falls back on a confidence below the threshold.

### Phase 8: Conductor integration

**Files:**
- `ARCMiS/lib/mas-core/src/conductor.rs` — wire all new modules
  into the conductor. Add `module_state_machines` field,
  `artifact_store` field, `advisor` field, `jev_service` field.
  Update `round` to dispatch module-level work. Update `run` to
  walk the two-level phase machine.
- `ARCMiS/lib/mas-core/tests/conductor.rs` — add tests for the
  two-level phase walk and module-level dispatch.

**Acceptance:** `cargo nextest run --profile ci -p ARCMiS-mas-core
--no-tests=pass` passes. The conductor runs the full phase walk
over a scripted transport with the example config.

### Phase 9: CLI integration

**Files:**
- `ARCMiS/bin/cli/src/main.rs` — pass the new config sections to
  the conductor.
- `ARCMiS/lib/run-config/src/schema.rs` — add `advisor` section,
  `jev` section.
- `configs/example.yml` — add the new sections.

**Acceptance:** `mise run cli -- configs/example.yml` runs the
full phase walk and reaches `Done` with `Proven` verdicts.

## Dependency graph

```
Phase 1 (Role registry)
    |
    v
Phase 2 (Phase machine)
    |
    v
Phase 3 (Task contract + Artifact store)
    |
    v
Phase 4 (Adaptive router)
    |
    v
Phase 5 (Verdicts)
    |
    v
Phase 6 (OMP advisor)
    |
    v
Phase 7 (JEV decision service)
    |
    v
Phase 8 (Conductor integration)
    |
    v
Phase 9 (CLI integration)
```

Each phase depends on the previous one. No phase can land before
its predecessor is green.

## Risk areas

1. **Backward compatibility.** The existing `Role`, `Phase`,
   `WorkItem`, `Blackboard`, `Router`, and `Verdict` types are
   public. Every existing variant and method must keep its name
   and behavior. Mitigation: add new variants and methods, never
   rename or remove existing ones. Add `From` impls for backward
   compatibility.

2. **Phase machine complexity.** The two-level phase machine
   (run-level plus per-module) is more complex than the current
   single-level machine. The conductor must coordinate two FSMs
   without races. Mitigation: the run-level FSM owns the module
   FSMs; module FSMs advance only when the run-level FSM permits.

3. **Artifact store validation.** The artifact store validates
   every artifact against its schema and validation rule. A
   schema mismatch or validation failure must not crash the run.
   Mitigation: validation failures return a `ValidationError`
   that the conductor logs and treats as a task failure.

4. **Router cascade correctness.** The four-layer router must
   produce a `RouteDecision` for every signal combination. A
   miss in one layer must fall through to the next layer without
   losing context. Mitigation: each layer returns `Option<RouteDecision>`;
   the conductor chains the layers.

5. **OMP advisor overhead.** The advisor runs a second transport
   session. The `syncBacklog` bound prevents the advisor from
   blocking the primary transport. Mitigation: the advisor runs
   on a separate task with a bounded channel.

6. **JEV decision service fidelity.** The laya checkpoint is
   fine-tuned on four workflows. Off-workflow questions produce
   garbage. Mitigation: the service hardcodes the trained
   question map; off-workflow questions return `Fallback`.

7. **Test coverage.** The refactor touches every module. Existing
   tests must continue to pass; new tests must cover the new
   behavior. Mitigation: each phase adds tests for its new
   behavior; the conductor integration phase adds end-to-end
   tests.

## Test strategy

- **Unit tests:** every new module gets a `tests/<module>.rs`
  file with tests for its public API. The existing tests stay
  green.
- **Integration tests:** the conductor integration phase adds
  end-to-end tests over a scripted transport. The tests cover
  the full phase walk, module-level dispatch, artifact
  validation, router cascade, verdict computation, advisor
  consultation, and JEV decision service.
- **E2E tests:** the CLI integration phase adds a smoke run
  with the example config. The run must reach `Done` with
  `Proven` verdicts.
- **Regression tests:** the existing `c15-gr-hierarchy-smoke`
  run must reproduce byte-identically when the hierarchy is
  disabled. The refactored conductor must not change the
  single-tier behavior.
