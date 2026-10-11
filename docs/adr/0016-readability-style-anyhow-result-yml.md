# 0016. Readability-first code style: plain paths, anyhow, struct init

- **Date:** 2026-09-18
- **Status:** accepted

## Context

The codebase carried four style habits that made code heavier to read than
to write:

- Fully qualified paths (`::std::clone::Clone::clone(&name)`) buried each
  item behind a wall of qualifiers. A method call says the same with less
  noise.
- Free functions named operations: `init_tracing()`, `monolith_task()`,
  `truncate_args()`. A struct with an `init` (or `build`) method names the
  concept and groups its knobs.
- `main` resolved every error inline with `match` + `ExitCode::FAILURE`.
  Seven `match` blocks in sequence. `anyhow` plus a fallible `main` says
  the same with `?`.
- `run-report.md` was a fixed name in the output dir. Runs of the same
  config overwrote the previous result.

## Decision

- `.omp/rules/rust.md` §1: plain paths. No leading `::` on expressions,
  types, `use` items, attributes, derives, or macros. §7 (new): write
  method calls, not fully qualified functions (`foo.clone()`, not
  `Clone::clone(foo)`).
- Fallible plumbing returns `anyhow::Result` and bubbles to a fallible
  `main`. Library domain errors stay typed. Tool input validation keeps
  `Result<_, String>` because the message is data for the model.
- Operations that name a concept become structs with methods
  (`Tracing::init`, `MonolithTask::build`) instead of free functions.
- The run result lands at `{output.dir}/.ARCMiS/result/{UTC
  timestamp}.yml`. Consecutive runs of one config never overwrite each
  other.

## Consequences

- Lint, clippy, and review now flag `::` prefixes instead of requiring
  them. This ADR supersedes the macro-qualification point of ADR 0010.
- `main` is `-> anyhow::Result<ExitCode>`. Failures log once and return
  `FAILURE`.
- One result file per run. Consumers glob the newest.
