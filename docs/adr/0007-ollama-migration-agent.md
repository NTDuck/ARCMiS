# 0007. Ollama-driven migration agent over GildedRose

- **Date:** 2026-09-14
- **Status:** accepted

## Context

The task: drive a migration agent that transforms `assets/GildedRose-Refactoring-Kata/C` into a Rust codebase under `.artifacts/GildedRose-Refactoring-Kata/Rust`. The ollama model is `openbmb/minicpm5-2b:q8_0`. Task-specific values must live in `assets/configs/GildedRose-Refactoring-Kata/config.yml`, not in the agent. The run must log reasoning, tool calls, and output. It must measure compile status plus a test pass rate, with tests translated as-is. Any compile status and pass rate is acceptable. The run must be reasonably fast.

## Decision

- One agent (`lib/agents/src/migration.rs`) runs on the pinned rig 0.42 facade with two tools (`lib/tools/src/migration.rs`): `write_file` and `run_command`. Source contents are pre-seeded into the prompt. In run 1 a read-first loop burned 12 of 12 turns exploring. The tool loop is now write, build, fix.
- The harness (`bin/harness`) writes a config-defined package skeleton into the output dir before the run. It drives the agent with a logging hook and measures after each attempt. A turn-budget exhaustion is a run end, not a fatal error. Whatever the agent wrote still gets measured. Attempts stop early on the first compiling result.
- All task-specific values live in `config.yml` per `.omp/rules/config.md`. The list: model, budgets, retries, thinking toggle, temperature, file lists, languages, test command, style hint, protected files, and scaffold. The agent contains no task values.
- Model-facing guardrails live in the tools, not prompts. Path sanitization rejects absolute paths and `..`. That guard fixes a real bug: `PathBuf::join` with an absolute path escaped the sandbox. Protected files fail closed with model feedback.

## Verified Model Facts (ollama, openbmb/minicpm5-2b:q8_0)

- The model runs away in think blocks on long prompts: 3/3 rolls of the exact failing request emitted 8192 empty tokens with `done_reason: length`. `think: false` fixed it: 3/3 clean stops with tool calls. Verified via direct ollama API probes.
- `num_ctx` must be a flat `additional_params` entry (the published rig ollama adapter merges it into ollama options). A nested `options` object is silently ignored.
- The model cannot reliably complete the translation. It fabricates absolute paths. It re-writes protected files after explicit feedback. It burns its budget on exploratory `ls` and `find` calls even with a compilable scaffold present. More than 15 runs confirm this. The harness therefore treats a failed agent attempt as a measured outcome, not a harness fault.

## Consequences

- The pipeline is honest and fast: it attempts, logs, measures, and reports compile status and pass rate whatever they are. Current typical run: about 90 seconds, compile=fail, no pass rate, full tool-call log.
- A stronger ollama model (or a config swap to a hosted provider) plugs into the same config file with zero code change.
- `.gitignore` excludes `.artifacts/`. The run report lands at `.artifacts/GildedRose-Refactoring-Kata/Rust/run-report.md`.
