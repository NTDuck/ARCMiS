# 0038. Role registry for Claude Code Modernization

- **Date:** 2026-10-12
- **Status:** accepted
- **Extends:** [0026](0026-orchestrator-rename-hierarchical-static-orchestration.md), [0036](0036-hierarchical-mas-claude-code-modernization.md)
- **Related:** [0037](0037-mas-core-refactor-implementation-plan.md)

## Context

The Claude Code Modernization workflow defines a fixed agent roster
with six leads and sixteen specialists. The current `Role` enum
holds ten specialists under three leads. The refactor must grow the
roster to twenty-two variants while keeping the existing ten
variants and their names byte-identical for backward compatibility.

## Decision

### 1. Complete role enum mapping

The `Role` enum grows from ten to twenty-two variants. The mapping
from old to new:

| Old role | New role | Tier | Lead |
| -------- | -------- | ---- | ---- |
| `Analyst` | `LegacyAnalyst` | 3 | `DiscoveryLead` |
| `Architect` | `TopologyAnalyst` | 3 | `DiscoveryLead` |
| `Planner` | `StrategySelector` | 3 | `PlanningLead` |
| `Translator` | `UpliftWorker` | 3 | `MigrationLead` |
| `Validator` | `IndependentReviewer` | 3 | `VerificationLead` |
| `Tester` | `DifferentialTester` | 3 | `VerificationLead` |
| `FailureAnalyst` | `FailureAnalyst` | 3 | `MigrationLead` |
| `Repairer` | `Repairer` | 3 | `MigrationLead` |
| `Critic` | `ArchitectureCritic` | 3 | `PlanningLead` |
| `FleetAnalyst` | `FleetAnalyst` | 1 | (advisory) |

New roles added by the refactor:

| New role | Tier | Lead |
| -------- | ---- | ---- |
| `TopologyAnalyst` | 3 | `DiscoveryLead` |
| `BusinessRulesExtractor` | 3 | `SemanticsLead` |
| `CitationAuditor` | 3 | `SemanticsLead` |
| `StrategySelector` | 3 | `PlanningLead` |
| `ArchitectureCritic` | 3 | `PlanningLead` |
| `TestEngineer` | 3 | `PlanningLead` |
| `ModuleScheduler` | 3 | `MigrationLead` |
| `UpliftWorker` | 3 | `MigrationLead` |
| `TransformWorker` | 3 | `MigrationLead` |
| `ReimagineWorker` | 3 | `MigrationLead` |
| `IndependentReviewer` | 3 | `VerificationLead` |
| `DifferentialTester` | 3 | `VerificationLead` |
| `ProofBuilder` | 3 | `VerificationLead` |
| `SecurityAuditor` | 3 | `SecurityLead` |
| `PatchReviewer` | 3 | `SecurityLead` |

New leads added by the refactor:

| New lead | Tier | Members |
| -------- | ---- | ------- |
| `DiscoveryLead` | 2 | `LegacyAnalyst`, `TopologyAnalyst` |
| `SemanticsLead` | 2 | `BusinessRulesExtractor`, `CitationAuditor` |
| `PlanningLead` | 2 | `StrategySelector`, `ArchitectureCritic`, `TestEngineer` |
| `MigrationLead` | 2 | `ModuleScheduler`, `UpliftWorker`, `TransformWorker`, `ReimagineWorker`, `TestEngineer`, `FailureAnalyst`, `Repairer` |
| `VerificationLead` | 2 | `IndependentReviewer`, `DifferentialTester`, `ProofBuilder` |
| `SecurityLead` | 2 | `SecurityAuditor`, `PatchReviewer` |

### 2. Team partitions

The `TeamRegistry` compiles six teams from the run config. Each
team is a whole-fleet partition: a specialist that sits under two
leads makes routing ambiguous. Preflight rejects it (ADR 0026).

The six teams:

1. **Discovery team** (`DiscoveryLead`): `LegacyAnalyst`,
   `TopologyAnalyst`. Reads the codebase, writes the source map
   and the topology graph.
2. **Semantics team** (`SemanticsLead`): `BusinessRulesExtractor`,
   `CitationAuditor`. Extracts business rules from the source
   code, audits citations against the source map.
3. **Planning team** (`PlanningLead`): `StrategySelector`,
   `ArchitectureCritic`, `TestEngineer`. Selects the migration
   strategy per module, critiques the architecture, designs the
   test plan.
4. **Migration team** (`MigrationLead`): `ModuleScheduler`,
   `UpliftWorker`, `TransformWorker`, `ReimagineWorker`,
   `TestEngineer`, `FailureAnalyst`, `Repairer`. Schedules
   modules, translates them under the selected strategy, runs
   tests, diagnoses failures, applies repairs.
5. **Verification team** (`VerificationLead`):
   `IndependentReviewer`, `DifferentialTester`, `ProofBuilder`.
   Reviews translations independently, runs differential tests,
   builds proofs of correctness.
6. **Security team** (`SecurityLead`): `SecurityAuditor`,
   `PatchReviewer`. Audits the translated code for security
   issues, reviews patches before they land.

### 3. Tier assignments

The tier assignments follow the Claude Code Modernization design:

- **Tier 1:** `Orchestrator` (the conductor itself), `FleetAnalyst`
  (advisory, no migration artifacts).
- **Tier 2:** `DiscoveryLead`, `SemanticsLead`, `PlanningLead`,
  `MigrationLead`, `VerificationLead`, `SecurityLead`.
- **Tier 3:** all specialists listed above.

The `FleetAnalyst` stays tier-1 accessible: it advises on model
promotion and produces no migration artifacts, so routing it
through a lead would add a hop without isolation value (ADR 0026).

### 4. Migration path for existing configs

Existing configs that name the old specialists continue to work
through a name alias table. The alias table maps old names to new
roles:

| Old name | New role |
| -------- | -------- |
| `analyst` | `LegacyAnalyst` |
| `architect` | `TopologyAnalyst` |
| `planner` | `StrategySelector` |
| `translator` | `UpliftWorker` |
| `validator` | `IndependentReviewer` |
| `tester` | `DifferentialTester` |
| `failure-analyst` | `FailureAnalyst` |
| `repairer` | `Repairer` |
| `critic` | `ArchitectureCritic` |
| `fleet-analyst` | `FleetAnalyst` |

The alias table is applied at `Role::from_name` time. A config
that names `analyst` resolves to `LegacyAnalyst`; a config that
names `legacy-analyst` also resolves to `LegacyAnalyst`. Both
forms are accepted.

Existing configs that name the old leads continue to work through
a lead alias table:

| Old lead name | New lead |
| ------------- | -------- |
| `discovery-lead` | `DiscoveryLead` |
| `migration-lead` | `MigrationLead` |
| `integration-lead` | `PlanningLead` |

The `integration-lead` alias maps to `PlanningLead` because the
old integration team's responsibilities (batch ordering, failure
diagnosis, adversarial review) split across the new planning,
migration, and verification teams. The alias preserves the old
config's intent: a config that names `integration-lead` gets a
team with `StrategySelector`, `ArchitectureCritic`, and
`TestEngineer` (the planning team's roster).

### 5. New config fields

The `TeamConfig` struct grows an optional `strategy` field:

```yaml
hierarchy:
  enabled: true
  teams:
    - lead: migration-lead
      members: [uplift-worker, transform-worker, reimagine-worker, test-engineer, failure-analyst, repairer]
      turns: 40
      stagnation_rounds: 3
      strategy: parallel
```

The `strategy` field names the migration strategy for the team:
`parallel` (concurrent dispatch for independent modules),
`topological` (sequential dispatch respecting dependencies),
`grouped` (grouping for cycles). The default is `parallel`. The
field is optional; absent means `parallel`.

### 6. Backward compatibility

The refactored `Role` enum keeps all ten old variants. The old
variants are marked `#[deprecated]` with a note pointing to the
new variant. The `From<OldRole> for Role` impl maps old to new.
Existing call sites that match on old variants continue to work;
the compiler emits a deprecation warning.

The `TeamRegistry::compile` function accepts both old and new
member names. A config that mixes old and new names compiles to a
team with the resolved new roles. A config that names only old
leads compiles to the aliased new teams.

## Consequences

- The `Role` enum grows from 10 to 22 variants. Every existing
  variant keeps its name and behavior. New variants add new
  capabilities without breaking existing configs.
- The `TeamRegistry` compiles six teams from the run config. A
  disabled hierarchy compiles to the direct registry exactly as
  today.
- The alias table preserves backward compatibility for existing
  configs. Old configs continue to work without changes.
- The `strategy` field adds a new dimension to team configuration.
  The default `parallel` matches the current behavior.
- The deprecation warnings guide operators to update their
  configs. The warnings are non-blocking; the refactored
  conductor accepts both old and new names.

## Verification

- Unit: `Role::from_name` resolves every old and new name to the
  correct variant.
- Unit: `TeamRegistry::compile` accepts the example config and
  produces six teams.
- Unit: `TeamRegistry::compile` accepts a config with old names
  and produces the aliased new teams.
- Unit: `TeamRegistry::validate_unique_memberships` rejects a
  config with duplicate memberships.
- Unit: the alias table maps every old name to the correct new
  role.
- Integration: the conductor runs the full phase walk over a
  scripted transport with the example config and reaches `Done`.
