# ADR 0022: Hierarchical dynamic-adaptive orchestration

Date: 2026-09-25
Status: Accepted (extended by ADR 0026, which adds the static hierarchy axis)

## Context

The current MAS is a single-tier orchestrator-worker loop: one manager
agent emits a `DECISION: delegate ROLE | TASK` line per round, a keyword
router resolves the role, and the harness executes exactly one delegation
before asking the manager again. The pattern matches the survey
"LLM-Based Multi-Agent Orchestration: A Survey of Frameworks,
Communication Protocols, and Emerging Patterns" taxonomy of a
centralized orchestrator-worker topology with a static pipeline axis:
the sequence of roles per phase is hardwired into the manager prompt and
the state machine, and the round loop cannot fan out, spawn
sub-collectives, or re-route mid-task on intermediate state.

Three forces motivate the change:

1. **Hierarchical dynamic-adaptive orchestration** (the survey's emerging
   pattern): multi-tier control where a top-tier manager handles
   macro-planning and delegates sub-collectives to lower-tier managers,
   with agents that are static (fixed registry, fixed preambles, fixed
   tool subsets) but whose *selection and composition at runtime is
   dynamic* — a router inspects intermediate state and picks the next
   actor, parallel swarms form for independent work, and strategy
   modulates on observed progress.
2. **Jev-as-a-guard**: the tool gateway must be an enforcement point on
   every tool call, not a document. The current `Guard` is constructed
   and dropped (`let _guard = ...`), `GuardedTool` is dead code, and the
   allowlist lives outside the tool path. The harness playbook rule the
   repo already follows — deterministic rules first, model arbitration
   only for the ambiguous remainder — needs to sit on `on_tool_call`.
3. **Snapcompact for all agents**: context archival currently builds a
   hook in `main.rs` and binds it to `_offload_hook`, an unused binding.
   Long-context specialists (translator, validator) are exactly the ones
   that need PNG-frame archival, and they get nothing.

ReCodeAgent contributes the four-stage pipeline discipline (its stages
map onto our phases; the pipeline stays as the default strategy, not a
hardcoded control flow), and the modernization plugin contributes the
discovery→brief→execution flow with evidence-gated phase exits.

## Decision

### 1. Static agents, dynamic selection (the pattern's core split)

Agents remain static: 10 specialists + manager, each one preamble, one
tool allowlist, one judge contract, built once by the registry. What
becomes dynamic is selection and composition at runtime:

- **Task graph replaces the single-delegation round.** The manager's
  `delegate` verb may emit multiple tasks in one round; the orchestrator
  executes them as a DAG: tasks with no unmet `depends_on` run in
  parallel (bounded by `config.mas.fanout`, default 2 — one ollama slot),
  and each task's role is picked by the router chain
  (manager hint → keyword pass → router model), not fixed by the phase.
- **Dynamic routing on intermediate state.** After each delegation the
  orchestrator re-scores the task list (progress.rs) and the manager's
  next round sees the merged state; a failing task can spawn a
  failure-analyst → repairer sub-chain without a phase regression. The
  keyword router and the router model both become fallbacks to the
  manager's hint rather than primary selectors.
- **Sub-collectives, one level deep.** A `delegate` whose task names a
  batch may declare `team=true`; the orchestrator then runs the
  well-known translation collective for that batch (translator →
  validator → tester, with repairer re-entry on fail) as an inner loop
  under one task id. The manager stays at tier 1; the collective is
  tier 2 with its own turn budget (`collective_turns`, default
  worker_turns) and its own breaker. Deeper nesting is refused by the
  guard (fan-out budget, not agent code).

### 2. Jev-as-a-guard on the tool path

The gateway moves into the rig hook stack (`on_tool_call`) so every call
from every agent is checked at the choke point:

- Deterministic tier first: role allowlist (from
  `ToolEnvelope::metadata` + `Role::allowed_tools`), path policy
  (`source/` read-only, workspace containment), deny patterns
  (`rm -rf`, `git push`, package installs, network), cost-class budget
  (expensive tools per-delegation). Violations return
  `ToolCallAction::Skip(feedback)` with the reason — the model sees the
  refusal and self-corrects; repeated violations trip the breaker.
- Model arbitration tier (the Jev slot): only for calls the
  deterministic tier marks `Ask` — today: `bash` commands outside the
  deny list but not on the role's read allowlist, and write paths that
  leave `target/`. Gated by `guard.ask_model: false` in the default
  config (the local model is the same one the caller runs; arbitration
  buys nothing until a second model exists). The seam is the
  `GuardArbiter` trait; the laya typed-decision crate is the intended
  implementer later.
- The old `Guard::permits_tool/permits_path` stay as pure functions the
  hook calls; `GuardedTool` is deleted (the hook replaces it).

### 3. Snapcompact for all agents

`SnapcompactHook` attaches in `registry::build` next to the trace and
continuation hooks, with per-role thresholds: manager and long-context
specialists (translator, validator, tester) get the configured
threshold; short-context roles get a higher one (archival rarely
triggers). The hook moves to `agents` so the registry owns it;
`snapcompact` stays a separate crate.

### 4. Stencil playbook philosophies kept

- **Prompts as versioned markdown** loaded with `include_str!`
  (unchanged).
- **Declarative tool metadata** (`envelope.rs`) as the guard's source of
  truth (now actually consumed by the hook).
- **Five edit modes reserved, hashline primary** (unchanged).
- **Catalog as a separate concern**: the fleet ladder stays config-
  driven; promotion stays the fleet-analyst's verb.
- **Determinism at the boundary**: phase exits remain evidence-gated
  (`phase_delegations`), the breaker remains config-driven, scoring
  remains toolchain-only.

## Consequences

- The manager prompt gains the multi-delegate and team verbs; the state
  machine, judge contracts, and evidence gates are unchanged.
- Round-loop parallelism is bounded by ollama's single-model reality:
  `fanout: 2` interleaves two specialists' turn streams; true
  parallelism arrives with a second model slot, not new code.
- `GuardedTool` and the dead `_guard` binding disappear; the hook is the
  single enforcement point.
- Snapcompact events appear in traces for every role, making context
  growth observable per agent.

## Verification

- Unit: guard hook denies an out-of-allowlist write and a `source/`
  edit; allows a `target/` write; `Ask` falls through to deny when the
  arbiter is absent.
- Unit: task-graph executor runs an independent pair in two rounds and
  a `depends_on` pair in order.
- E2E: GildedRose C→Rust run with `fanout: 2` completes with the
  toolchain green and traces show snapcompact events on the manager.
