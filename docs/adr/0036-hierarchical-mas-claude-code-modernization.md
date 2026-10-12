# 0036. Hierarchical MAS for Claude Code Modernization

- **Date:** 2026-10-12
- **Status:** accepted
- **Extends:** [0022](0022-hierarchical-dynamic-adaptive-orchestration.md), [0026](0026-orchestrator-rename-hierarchical-static-orchestration.md), [0027](0027-laya-typed-triage-on-lead-dispatch.md), [0031](0031-mas-core-conductor-over-omp-transport.md)
- **Related:** [0037](0037-mas-core-refactor-implementation-plan.md), [0038](0038-role-registry-claude-code-modernization.md), [0039](0039-two-level-phase-machine.md), [0040](0040-task-contract-and-artifact-store.md)

## Context

The current `mas-core` conductor (ADR 0031) implements a three-tier
hierarchy with three leads (`discovery-lead`, `migration-lead`,
`integration-lead`) over ten specialists, a single phase machine
(Preflight through FinalValidation), a simple task graph, a keyword
router with laya triage, and a topic-revision blackboard. The Claude
Code Modernization workflow demands a richer structure: six leads
covering discovery, semantics, planning, migration, verification, and
security; a two-level phase machine (run-level plus per-module); typed
task contracts with provenance; an artifact store with owner and
validation rules; an adaptive router that combines deterministic
signals, repository metadata, test outcomes, and bounded model
judgments; recomputed verdicts (PROVEN, PARTLY_PROVEN, NOT_PROVEN);
and an OMP advisor integration with a bounded syncBacklog.

The current conductor cannot express these requirements without
breaking its public surface. The role enum holds ten specialists; the
phase enum holds ten phases; the task graph holds a `Kind` enum with
seven variants; the blackboard holds topic revisions without owner or
schema; the router holds a single keyword layer plus one laya
question. The refactor must grow every one of these surfaces while
keeping the existing single-tier behavior byte-identical when the
hierarchy is disabled (ADR 0026).

## Decision

`mas-core` grows into a Claude Code Modernization conductor with six
new modules and four extended modules. The conductor owns one run
lifetime, one transport, one task graph, one phase machine, one
artifact store, one router, one gateway, and one blackboard. Every
component compiles once per run from the run configuration and stays
immutable for the run's duration.

### 1. Role registry (ADR 0038)

The `Role` enum grows from ten to twenty-two variants. Six new leads
join the existing tier-2 set: `discovery-lead`, `semantics-lead`,
`planning-lead`, `migration-lead`, `verification-lead`,
`security-lead`. Twelve new specialists join the existing tier-3 set:
`legacy-analyst`, `topology-analyst`, `business-rules-extractor`,
`citation-auditor`, `strategy-selector`, `architecture-critic`,
`test-engineer`, `module-scheduler`, `uplift-worker`,
`transform-worker`, `reimagine-worker`, `independent-reviewer`,
`differential-tester`, `proof-builder`, `security-auditor`,
`patch-reviewer`. The existing ten specialists stay for backward
compatibility. The `TeamRegistry` compiles six teams from the run
config; a disabled hierarchy compiles to the direct registry exactly
as today.

### 2. Two-level phase machine (ADR 0039)

The run-level phase enum grows from ten to fourteen states: `Init`,
`Preflight`, `Assess`, `Map`, `ExtractRules`, `ReviewRules`,
`DraftBrief`, `MigratePilot`, `VerifyPilot`, `MigrateBatches`,
`VerifyModule`, `FinalVerify`, `Harden`, `HumanSignOff`, `Done`. A
new `ModulePhase` enum carries the per-module FSM: `Planned`,
`Ready`, `Baselined`, `Migrating`, `Verifying`, `Proven`, with
failure transitions `Diagnosing`, `Repairing`, `Escalated`. The
`PhaseMachine` keeps the run-level FSM; a new `ModuleStateMachine`
holds one FSM per module. Evidence gates move from a single
`Evidence` enum to a per-transition gate function table.

### 3. Task contracts and artifact store (ADR 0040)

The `WorkItem` struct grows into a `TaskContract` with structured
fields: `task_id`, `phase`, `unit`, `strategy`, `risk`,
`required_role`, `required_inputs` (with content hashes),
`write_scope`, `max_attempts`, `verification_required`. A new
`TaskResult` struct carries the outcome: `task_id`, `verdict`,
`artifacts` (with provenance), `evidence`, `attempts_used`,
`failure_class`. The blackboard grows into an `ArtifactStore` with
owner, schema, provenance, and validation rule per artifact. The
existing topic-revision API stays as a thin wrapper over the
artifact store for backward compatibility.

### 4. Adaptive router

The `Router` grows from a keyword-plus-laya layer to a four-layer
cascade: deterministic signals first (phase, role, unit, risk),
repository metadata second (file topology, dependency graph, test
coverage), test outcomes third (pass/fail counts, coverage deltas,
regression markers), bounded model judgments last (laya typed
decisions at the pre-identified bottlenecks). The router returns a
`RouteDecision` with the target role, the basis, and the confidence.
Routing decisions cover concurrent dispatch for independent modules,
topological order for dependencies, grouping for cycles, and
retry/diagnose/escalate for failures.

### 5. Verdicts

The `Verdict` enum grows from `Success`/`Failed` to three recomputed
states: `Proven`, `PartlyProven`, `NotProven`. Verdicts recompute
from current evidence on every phase advance and every module
transition. A `VerdictComputer` function takes the artifact store
and the test outcomes and returns the verdict for one module or the
whole run.

### 6. OMP advisor integration

The conductor grows an `OmpAdvisor` handle that wraps a second
`OmpTransport` configured for read-only consultation. The advisor
shares the same model as the primary transport (no second model
slot). The advisor's `syncBacklog` is bounded by the configured
`advisor_backlog` limit (default 16). The advisor answers
classification questions the deterministic tier marks `Ask` and the
laya tier marks `Fallback`.

### 7. JEV decision model service

The laya triage wrapper grows from one question (`verdict`) to a
typed decision service covering four decision classes: failure
triage, next-handler selection, escalation priority, and retry
strategy. Each class maps to a trained workflow from the
`agent_trace_observability` split. The cascade rule, the confidence
statistic, and the local-checkpoint policy carry over from ADR 0027.

## Consequences

- The `Role` enum grows from 10 to 22 variants. Every existing
  variant keeps its name and behavior. New variants add new
  capabilities without breaking existing configs.
- The `Phase` enum grows from 10 to 14 variants. The existing
  variants keep their names and forward edges. New variants add
  new phases between existing ones.
- The `WorkItem` struct grows into `TaskContract`. Existing call
  sites that build a `WorkItem` from a `Kind` and an instruction
  continue to work through a `From` impl.
- The `Blackboard` grows into an `ArtifactStore`. Existing topic
  revision calls continue to work through a thin wrapper.
- The `Router` grows from two layers to four. Existing keyword
  rules continue to work; new layers add new signals.
- The `Verdict` enum grows from two variants to three. Existing
  `Success` and `Failed` variants map to `Proven` and `NotProven`
  respectively.
- The conductor grows an `OmpAdvisor` handle. Existing runs that
  do not configure an advisor see no change.
- The laya triage grows from one question to four. Existing
  `verdict` consultations continue to work; new consultations add
  new decision classes.
- The refactor touches every module in `mas-core`. The change set
  is large but mechanical: every module grows new fields and
  methods without breaking its existing public surface.
- The smoke run `c15-gr-hierarchy-smoke` (GildedRose, parents
  `20260928T173947Zc10-c10b-oxidizer-checkdigit`) reached Migration
  in 13 rounds with 5 tier-2 handoffs and 14 tier-3 dispatches.
  The refactored conductor must reproduce this run byte-identically
  when the hierarchy is disabled.

## Verification

- Unit: the role registry compiles six teams from the example
  config and rejects duplicate memberships.
- Unit: the phase machine walks the full forward path from `Init`
  to `Done` and rejects illegal transitions.
- Unit: the module state machine walks the full forward path from
  `Planned` to `Proven` and exercises every failure transition.
- Unit: the task contract builder produces a valid contract from
  the example config and rejects missing required fields.
- Unit: the artifact store accepts a typed artifact with owner,
  schema, provenance, and validation rule, and rejects an artifact
  that fails its validation rule.
- Unit: the router returns a `RouteDecision` for every signal
  combination and falls through to the next layer on a miss.
- Unit: the verdict computer returns `Proven` for a module with
  green tests and complete artifacts, `PartlyProven` for a module
  with green tests and missing artifacts, and `NotProven` for a
  module with red tests.
- Unit: the OMP advisor accepts a read-only consultation and
  rejects a write consultation.
- Unit: the JEV decision service answers all four decision classes
  and falls back on a confidence below the threshold.
- Integration: the conductor runs the full phase walk over a
  scripted transport with the example config and reaches `Done`
  with `Proven` verdicts on every module.
- E2E: the GildedRose C-to-Rust run with the example config
  completes with the toolchain green and traces show the new
  phase names and role names.
