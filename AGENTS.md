# ARCMiS — Agent Guide

`AGENTS.md` is the map, not the manual. Start here, then follow pointers to deeper sources of truth.

- **Source of record for decisions:** [`docs/adr/`](docs/adr/README.md) — numbered, immutable ADRs; supersede, never rewrite.
- **Workflow rules (binding):** [`.omp/rules/`](.omp/rules/) — conventional-commit granularity, decision logging, Rust style.
- **Style/lint config:** [`.rustfmt.toml`](.rustfmt.toml) (nightly-oriented; stable ignores the unstable subset), [`.clippy.toml`](.clippy.toml), [`.config/nextest.toml`](.config/nextest.toml).
- **CI:** [`.github/workflows/`](.github/workflows/) — lint → build → test → dependencies-check (fmt/clippy on nightly, matrix test via nextest, cargo-deny + cargo-audit).

## Working Rules

1. Commits: granular, incremental, [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). See `.omp/rules/granular-conventional-commits.md`.
2. Every historical decision not directly inferrable in code gets a WHY comment at the site **and** an ADR in `docs/adr/`. See `.omp/rules/decision-logging.md`.
3. Rust style: fully qualified paths and derives, tap-chained instead of nested calls, functional style within functions. See `.omp/rules/rust-style.md`.
4. Verify before commit: `cargo fmt --check`, `cargo clippy`, `cargo test` (aliases: `cargo lint-rustfmt`, `cargo lint-clippy`). Never commit a red tree.
5. Tests run through `cargo nextest` locally and in CI.

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo nextest run --profile ci --workspace --all-targets --no-tests=pass
```
