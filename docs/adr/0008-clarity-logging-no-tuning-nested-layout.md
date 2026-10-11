# 0008. Code clarity, minimal code, logging, no tuning, nested layout

- **Date:** 2026-09-14
- **Status:** accepted

## Context

The first working migration pipeline carried four debts. Utility code sat next to tool definitions. The tool list was not visible at the module root. The binary printed to stdout with `println!` and lost levels, fields, and async safety. Several constants and prompt texts tuned against the GildedRose instance: per-file caps, protected-file lists, scaffold contents. The workspace members also sat at the repo root while the repository itself is ARCMiS, so the tree read `ARCMiS/lib/agents` as `lib/agents`.

## Decision

- Layout: workspace members move to `ARCMiS/lib/*` and `ARCMiS/bin/*`. Root manifest members are `ARCMiS/lib/*`, `ARCMiS/bin/*`.
- Three new rules: `.omp/rules/code-clarity.md`: the module opens with the list of things, per-item small named functions, no mixed abstraction levels. `.omp/rules/minimal-code.md`: least code, prefer external crates, why comments for hand-rolled general capabilities. `.omp/rules/logging.md`: no print. Tracing only, subscriber only in the binary, structured fields, STE-clean messages.
- Two rule extensions in `.omp/rules/rust.md`. Explicit variable names (no `e`, `o`, `c`). Turbofish type resolution for local bindings.
- One new rule: `.omp/rules/no-tuning.md`. No hand-tuning against the problem set. Run-specific values live only in the config file. Verified model facts become documented defaults, not hidden switches.
- The migration agent becomes the default agent (`ARCMiS/lib/agents/src/agent/default.rs`).
- Mechanical gate for the style rule: `.omp/scripts/lint-fqpn.py` fails on unqualified external `use` and unqualified derive paths. Run it before every commit. It lives under `.omp/scripts/`.

## Consequences

- The agent generates the output package itself, manifest and directories included. The config carries only the input codebase, the output dir, languages and toolchains, and run configs. Source file lists, scaffolds, style hints, and protected-file lists are gone from config and code.
- Harness no longer writes scaffolding. It discovers input files generically: walk the input root, keep text-decodable files, apply a stated per-file cap.
- All diagnostic output flows through tracing. The workspace contains zero `println!`.
- The fqpn gate is advisory tooling like the STE linter. The commits rule still governs what may be committed.
