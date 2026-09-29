# 0026. Orchestrator rename and hierarchical static orchestration

- **Date:** 2026-09-29
- **Status:** accepted
- **Extends:** [0022](0022-hierarchical-dynamic-adaptive-orchestration.md). The
  dynamic-selection layer stays. This ADR adds the static hierarchy axis.
- **Supersedes:** nothing (ADR 0022 remains accepted)

## Context

ADR 0022 built a single-tier MAS: one orchestrator round loop, a task
graph, and one-level team collectives. Two gaps remained:

1. The tier-1 agent carried the name "manager" (`Role::Manager`,
   `prompts/manager.md`, `manager.rs`, `manager_turns` config keys). The
   crate that hosts it is `orchestrator`. The split naming made
   traces, configs, and code disagree.
2. The phase machine (Discovery through FinalValidation) forces the
   orchestrator to switch between macro work (plan, contract, gates) and
   micro work (dispatch one translator on one batch) in the same round
   loop. Context for both jobs lives in one history, and the orchestrator
   turn budget pays for both.

## Decision

Two changes.

### Rename: manager -> orchestrator

`Role::Manager` becomes `Role::Orchestrator`, `prompts/manager.md`
becomes `prompts/orchestrator.md`, and `ManagerLoop` becomes
`OrchestratorLoop`. The crate name was already `orchestrator`, so the
loop type keeps the module-qualified short name: a caller writes
`orchestrator::OrchestratorLoop`, and a `OrchestratorLoop` rename to
`Loop` would read bare. Config keys rename with the role
(`mas.orchestrator_turns`). Old run directories stay untouched, and the
parsers in `scripts/` read role names additively, so old traces parse.

### Hierarchy: tier-2 leads over tier-3 specialists

Add a static three-tier tree, config-owned under `mas.hierarchy`:

- `enabled` (default `false`): off reproduces the single-tier behavior
  byte for byte. The dispatch path returns `Direct` before any team
  lookup.
- `deny_direct` (default `true`): the gate refuses a tier-1 delegation
  that names a team specialist. The refusal carries gate feedback instead
  of a silent reroute. The refusal teaches the orchestrator the tier
  split.
- `teams`: one entry per lead (`lead`, `members`, `turns`,
  `stagnation_rounds`). The config owns the split. The preflight
  validator rejects a member that sits in two teams (routing would be
  ambiguous) and a lead without a prompt file.

The tier handoff is context management. A lead receives a scoped brief
(task text, run contract, plan and notes tails under the existing
`plan_cap`/`notes_cap` caps), not the orchestrator's history. The lead's
inner loop parses `DECISION: delegate MEMBER | task` lines, dispatches
one member per round through the same `run_single` path as tier 1, and
ends on `done`, its turn budget, its stagnation breaker, or a model
failure. Its result folds into one `TaskResult` for the orchestrator.

Depth cap 3: the lead parser allows only its own member roles, and lead
names are not `Role`s, so a lead cannot dispatch another lead. A
cross-team or lead-named dispatch draws a logged refusal and counts
toward the stagnation breaker.

Tier-1 round prompts list the lead names in `ROLE` when hierarchy is on,
so `DECISION: delegate migration-lead | ...` is the handoff form.

### Split criterion: middleware vs domain (with 0025)

A lead agent is domain, not middleware: it wraps a team, a prompt, and a
budget, all owned by the config, and it exists only inside the MAS run
loop. The hooks stayed in `middleware` (0025). The lead building lives in
`agents` (`mas/leads.rs`, `registry::build_leads`), and the lead inner
loop lives in `orchestrator` (`lead.rs`), beside the tier-1 loop it
mirrors.

### Roles and traces

Lead roles and their teams, justified by the phase machine:

- `discovery-lead` over `{analyst, architect}`: Discovery and Contract
  produce the source map and brief.
- `migration-lead` over `{translator, validator, tester, repairer}`:
  Planning, Pilot, and Migration run the translate/validate/repair
  cycle. Most model calls land here.
- `integration-lead` over `{planner, failure-analyst, critic}`:
  Integration and Hardening run batch ordering, diagnosis, and the
  adversarial review.

`fleet-analyst` stays tier-1 accessible: it advises on model promotion,
and it produces no migration artifacts, so routing it through a lead
would add a hop without isolation value.

Delegation events carry the delegating tier (`tier: 1|2|3`) in the
ledger decision detail, the lead batch observation, and every trace
record (`TraceHook` names the agent and its tier). The fields are
additive. Readers that ignore them keep parsing old traces. Specialist
trace records stay at tier 1 in single-tier runs (they execute as direct
delegations) and read tier 3 only under a lead, so the tier is a fact
about the dispatch path, not about the role.

## Consequences

- A hierarchy run spends more orchestrator-visible turns per batch but
  keeps batch churn out of the tier-1 history. The lead's own budget
  absorbs retry loops.
- The refusal gate makes the hierarchy observable in the failure ledger,
  not only in traces.
- Old configs without a `mas.hierarchy` section deserialize with
  `enabled: false` and keep their old behavior. No config must change.
- The smoke run `c15-gr-hierarchy-smoke` (GildedRose, parents
  `20260928T173947Zc10-c10b-oxidizer-checkdigit`) reached Migration in 13
  rounds with 5 tier-2 handoffs and 14 tier-3 dispatches. Trace records
  show `discovery-lead`/`migration-lead` at tier 2 and specialists under
  them.
