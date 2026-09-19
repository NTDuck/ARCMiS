# 0017. Rig-idiom corpus style: bare prelude names, struct-namespace agents, shared task helper

- **Date:** 2026-09-19
- **Status:** accepted

## Context

ADR 0016 removed leading `::` prefixes, but the tree still carried the fully
qualified path style of ADR 0010: `std::string::String`,
`core::option::Option::Some`, `std::format!`, `core::result::Result::Ok`
appeared hundreds of times. The rig skill corpus (`0xPlaygrounds/rig`
`examples/`, pinned at commit `9b94481`) shows the upstream idiom: bare
prelude names, method-form prompting (`agent.prompt(x).max_turns(n)`), hook
action constructors (`CompletionCallAction::continue_run()`,
`ToolCallAction::run()`), and one struct namespace per agent
(`client.agent(...)` builder chains). The agents also duplicated the
serialize, prompt, parse sequence, and the harness hook used the enum
variants where the corpus uses constructors.

## Decision

- rust.md §1 grows the mechanical scope: every `std::*`/`core::*` prelude
  path (`std::string::String`, `core::option::Option::Some`,
  `std::format!`, `core::result::Result::Ok`, `std::vec::Vec`, and the rest
  of the prelude set) fails `lint-rules.py`. `use std::path::Path;` stays
  legal for non-prelude imports. The detector skips `use` lines.
- `pin!` joins the linter's prelude macro list (std prelude since Rust
  1.68).
- The two agents expose `Monolith::build`/`Monolith::run` and
  `Validator::build`/`Validator::run` instead of free `build`/`run`
  functions. The shape matches the harness helpers (`Tracing`,
  `MonolithTask`, `RunResult`) and rust.md §8. The free functions are gone.
  `lib.rs` re-exports the structs.
- One shared helper `agents::util::task::task<Response>(agent, task,
  max_turns)` owns the serialize, prompt, parse sequence. Both agent `run`
  methods delegate to it. The untyped `output_schema::<T>()` +
  `OutputMode::Tool` path stays: rig-agent 0.42 `TypedPromptRequest` pins
  `OutputMode::Native`, which breaks tool-composing structured output on
  ollama (see rig-agent 0.42 `agent/prompt_request/mod.rs`).
- The harness hook uses `CompletionCallAction::continue_run()` and
  `ToolCallAction::run()` per the corpus (`request_hook`).
- Manifests drop dead dependencies: workspace `bon`, tools crate `ignore`,
  `libc`, `grep-searcher`, `serde_yaml`.

## Consequences

- The linter now blocks the old style mechanically. A future contributor
  who writes `core::option::Option::Some` gets a hard failure.
- Agent call sites read `agents::Monolith::build(...)`. Re-exports in
  `agents/src/lib.rs` changed with it.
- The placeholder `Catalog` and `Registry` types, their cucumber suites,
  and ADR 0006/0009 test-contract wording stay untouched. Removing them is
  a separate structural decision, not part of this style pass.
