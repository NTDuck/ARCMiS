# ARCMiS — Agent Guide

`AGENTS.md` is the map, not the manual. Start here, then follow pointers to deeper sources of truth.

- **Workflow rules (binding):** [`.omp/rules/`](.omp/rules/) — commits, code clarity, config, decision logging, logging, minimal code, no tuning, Rust style, skill-corpus usage, nextest, STE.
- **Workspace layout:** members at `ARCMiS/lib/*` and `ARCMiS/bin/*` (ADR 0008).
- **Framework skill:** [`.omp/skills/rig/`](.omp/skills/rig/SKILL.md) — thin router + verbatim upstream `references/` corpus (examples from `0xPlaygrounds/rig`, pinned commit).
- **Enforcement scripts:** `.omp/scripts/lint-rules.py` (Rust style, naming, layout, logging, clarity) and `~/.omp/agent/skills/asd-ste100/scripts/ste-lint.py` (STE).
- **CI:** [`.github/workflows/`](.github/workflows/) — lint → build → test → dependencies-check (fmt/clippy on nightly, matrix test via nextest, cargo-deny + cargo-audit).

## Working Rules

1. Commits: granular, incremental, [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). See `.omp/rules/commits.md`.
2. Every historical decision not directly inferrable in code gets a WHY comment at the site **and** an ADR in `docs/adr/`. See `.omp/rules/decisions.md`.
3. Rust style: fully qualified paths, derives, and macros. Caller before callee. Tap-chained instead of nested calls. Functional style within functions. See `.omp/rules/rust.md`.
4. Run the gates before you commit: `cargo fmt --check`, `cargo clippy`, `cargo nextest run` (see `.omp/rules/build-and-gates.md`), and `python3 .omp/scripts/lint-rules.py` (qualified paths, derives, naming, print). Mise tasks: `mise run lint-rustfmt`, `mise run lint-clippy`. Never commit a red tree.
5. Tests: unit runs through `cargo nextest` locally and in CI. Behavior tests are cucumber suites under `{crate}/tests/features/`. Run them with `cargo test --features cucumber-tests --workspace --test cucumber`. See `.omp/rules/build-and-gates.md` and `docs/adr/0006-workspace-layout-and-cucumber-tests.md`.
7. Code clarity: the module opens with the list of things. User-facing code comes first. Callees follow their caller, depth-first. Each item reads as small named functions. Keep description strings pure. Do not mix abstraction levels. See `.omp/rules/code-clarity.md`.
8. Minimal code: prefer external crates. Do not re-implement general capabilities without a why comment. See `.omp/rules/minimal-code.md`.
9. No print. Log through `tracing` with structured fields. Keep messages STE-clean. See `.omp/rules/logging.md`.
10. Do not hand-tune against the problem set. Run-specific values live only in the config file. See `.omp/rules/no-tuning.md`.
11. Do not hardcode configuration. Bubble every setting up to the call site (`main` or the test entry). See `.omp/rules/config.md`.
12. Write all messages, comments, and wordings per the `asd-ste100` skill. Lint touched files before commit. See `.omp/rules/ste.md`.

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo nextest run --profile ci --workspace --all-targets --no-tests=pass
