# 0018. Multi-method agents: Ledger and Recode alongside Monolith

- **Date:** 2026-09-19
- **Status:** accepted

## Context

The agents crate hosted one migration method: `Monolith`, one agent that
translates the whole codebase in one tool loop. Two papers define
multi-agent alternatives worth benchmarking on the same task:

- arXiv:2608.26480 — a dynamic LLM-driven manager-worker loop over a shared
  filesystem workspace. Two agent roles only: manager and worker. Zero-shot
  self-orchestration: every role is the same model in a fresh context. The
  manager delegates tasks through the worker tool and re-curates the task
  list in the same loop. Workers write and build in the workspace.
- arXiv:2604.07341 (ReCodeAgent) — four specialized agents (analyzer,
  planning, translator, validator) orchestrated by deterministic scaffold
  code per Algorithm 1. The analyzer researches the target and designs the
  migration. The planner produces fragments, a name mapping, a skeleton,
  and a Part A/Part B plan. The translator executes the plan and repairs
  from the validation report. The validator runs tests and reports
  coverage gaps. The loop runs up to five iterations and stops early on
  full success.

## Decision

- The crate hosts three methods, each a struct namespace with `build` and
  `run`: `Monolith`, `Ledger`, `Recode`. The harness selects the method from
  the config directory name.
- `Ledger` and `Recode` reuse `MonolithRequest` and `MonolithResponse` as
  the common translation DTO pair. The harness dispatch stays uniform: one
  request in, one response out, regardless of method.
- Each method owns its own coordination result DTO: `LedgerResponse` and
  `RecodeResponse`. They mirror `ValidatorResponse` fields plus the method's
  own counters.
- The `Ledger` manager carries its worker through rig's
  `Agent::into_tool()` dynamic tool. Every role therefore runs in a fresh
  context per call, which the paper relies on. The worker tool name
  matches the agent name (`ledger_worker`). The manager delegates tasks
  and re-curates the task list inside one LLM-driven loop.
- The `Recode` phases run through deterministic scaffold code in
  `Recode::run`. Each phase call starts a fresh context. No LLM manager
  coordinates the phases.
- Config files under `assets/configs/<method>/` select model, budgets, and
  paths per run. The run sequence (translation then validation) stays for
  the monolith config. The new methods report their own validation in their
  response, because their loops already run the toolchain inside the
  workspace.

## Consequences

- The harness grows a method switch. Config directory names
  `ledger-method` and `ReCodeAgent-method` map to the new methods. Unknown
  names fall back to the monolith sequence.
- Benchmarks over the three methods share the task, the model, and the
  source section, so run-to-run deltas attribute to the method.
- The `Validator` agent stays in the crate. It serves the monolith run and
  remains the reference for validation DTO shapes.
