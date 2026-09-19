# 0018. Multi-method agents: Ledger and Recode alongside Monolith

- **Date:** 2026-09-19
- **Status:** accepted

## Context

The agents crate hosted one migration method: `Monolith`, one agent that
translates the whole codebase in one tool loop. Two papers define
multi-agent alternatives worth benchmarking on the same task:

- arXiv:2608.26480 — a manager-worker scaffold over a shared filesystem
  workspace. Zero-shot self-orchestration: every role is the same model in a
  fresh context, the manager curates tasks and delegates, workers write and
  build in the workspace.
- arXiv:2604.07341 (ReCodeAgent) — a language-agnostic repository
  translation pipeline: analyzer, planning, translator, and validator
  phases, coordinated through tool calls, with iterative repair.

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
- Worker agents ride into their manager through rig's `Agent::into_tool()`
  dynamic tool. Every role therefore runs in a fresh context per call, which
  is the mechanism both papers rely on. The worker tool name matches the
  agent name (`ledger_worker`, `recode_worker`).
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
