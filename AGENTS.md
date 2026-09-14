# ARCMiS — Agent Guide

`AGENTS.md` is the map, not the manual. Start here, then follow pointers to deeper sources of truth.

- **Source of record for decisions:** [`docs/adr/`](docs/adr/README.md) — numbered, immutable ADRs. Supersede a record. Never rewrite it.
- **Workflow rules (binding):** [`.omp/rules/`](.omp/rules/) — commits, decision logging, Rust style, distilled-skill usage, nextest.
- **Distilled framework skills:** [`.omp/skills/rig/`](.omp/skills/rig/SKILL.md) — AREX-style skill graph for the rig Rust LLM framework (router + 7 sub-skills, pinned upstream commit).
- **CI:** [`.github/workflows/`](.github/workflows/) — lint → build → test → dependencies-check (fmt/clippy on nightly, matrix test via nextest, cargo-deny + cargo-audit).

## Working Rules

1. Commits: granular, incremental, [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). See `.omp/rules/commits.md`.
2. Every historical decision not directly inferrable in code gets a WHY comment at the site **and** an ADR in `docs/adr/`. See `.omp/rules/decisions.md`.
3. Rust style: fully qualified paths and derives, tap-chained instead of nested calls, functional style within functions. See `.omp/rules/rust.md`.
4. Run the gates before you commit: `cargo fmt --check`, `cargo clippy`, and `cargo nextest run` (see `.omp/rules/use-nextest.md`). Mise tasks: `mise run lint-rustfmt`, `mise run lint-clippy`. Never commit a red tree.
5. Tests run through `cargo nextest` locally and in CI.
6. Tasks touching a distilled framework (e.g. rig): route through its `.omp/skills/` graph first and ground every upstream symbol before writing. See `.omp/rules/skills.md`.

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo nextest run --profile ci --workspace --all-targets --no-tests=pass
```
