---
description: Build and commit gates. Run Rust tests only through cargo-nextest. Run every gate before commit. Never commit a red tree.
---

# Build and Gates

This rule governs builds, tests, and gates for this repository. Review enforces the test-runner rule. Commit granularity is a separate rule (see `.omp/rules/commits.md`).

## 1. Test Runner: cargo-nextest Only

- Run all Rust tests through `cargo-nextest` (installed locally, 0.9.143). Never use `cargo test` to run tests.
- Command: `cargo nextest run --profile ci --workspace --all-targets --no-tests=pass`.
- CI (`.github/workflows/test.yml`) runs the same command. Keep both surfaces on nextest.
- Do not replace this command with `cargo test`. Nextest runs each test in its own process. It gives per-test isolation, better output, and stable retries.
- `--no-tests=pass` keeps empty crates green instead of failing the run.
- Exception: if you need a cargo test feature that nextest cannot run (for example doctests), run that one thing with `cargo test` and name the exception. Nextest does not run doctests.
- Enforcement: enforced in review.

## 2. Gates

Run every gate before you commit.

| Gate | Command | Mise task |
|---|---|---|
| Format | `cargo fmt --all` (check with `cargo fmt --check`) | `mise run lint-rustfmt` |
| Lint | `cargo clippy --workspace --all-targets` | `mise run lint-clippy` |
| Tests | `cargo nextest run --profile ci --workspace --all-targets --no-tests=pass` | — |
| Rule linter | `python3 .omp/scripts/lint-rules.py` (also a CI step in `.github/workflows/lint.yml`) | — |
| Prose | `python3 ~/.omp/agent/skills/asd-ste100/scripts/ste-lint.py <touched files>` | — |

Notes:

- `python3 .omp/scripts/lint-rules.py` (also a CI step in `.github/workflows/lint.yml`) covers fully qualified paths and derives (see `.omp/rules/rust.md`). No separate fqpn gate exists.
- The STE linter must report 0 hard violations. See `.omp/rules/ste.md`.
- The mise tasks replace the cargo aliases that lived in `.cargo/config.toml`. See `docs/adr/0004`.

## 3. Red Tree

- Never commit a red tree. If a gate fails, fix the code first. Then commit.
