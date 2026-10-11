# 0019. One folder per method

- **Date:** 2026-09-19
- **Status:** accepted

## Context

The two new methods started as single files: `ledger.rs` and `recode.rs`.
The Ledger method fits one file. The Recode method outgrew one file: it
carries four agents (analyzer, planner, translator, validator), a final
reporter, and the deterministic orchestration loop of ADR 0018. A single
file mixes agent definitions with scaffold code, and every reader must
scroll through unrelated agents to find the loop.

## Decision

- Each method lives in its own module folder
  `ARCMiS/lib/agents/src/<method>/`. The folder replaces the flat file.
- `mod.rs` owns the public namespace and the orchestration. The `run`
  function of the method struct stays in `mod.rs`, so the paper's
  orchestration pattern is readable in one place.
- One agent per file. The inner fold structure is
  `ledger/{mod,manager,worker}.rs` and
  `recode/{mod,analyzer,planner,translator,validator,reporter}.rs`.
  Sibling modules resolve through the folder namespace, for example
  `validator::ValidationReport` inside `recode::mod`.
- The hook default is the shared `agents::util::noop_hook::NoopHook`.
  Agent modules do not define private hook copies.

## Consequences

- Harness imports stay unchanged. `lib.rs` re-exports the method
  structs from the folders, so `agents::Ledger::build` and
  `agents::Recode::run` call sites keep their shape.
- Each agent module is unit-testable on its own. The file boundary matches
  the agent boundary, so a test targets one role at a time.
- Method orchestration is readable in one place: the `run` function in
  `mod.rs` shows the phase order and the iteration bound without a
  detour through agent definitions.
