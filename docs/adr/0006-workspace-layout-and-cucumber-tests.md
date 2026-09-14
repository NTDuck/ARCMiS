# 0006. Workspace layout and cucumber integration tests

- **Date:** 2026-09-14
- **Status:** accepted

## Context

The repository was a single crate (`src/lib.rs`). ARCMiS grows into several domains (agents, tools) plus a binary. Two forces shaped the layout. First, domains must depend in one direction only (agents may use tools, never the reverse), and a workspace enforces that with explicit path dependencies. Second, tests must not sit next to source: in-module `#[cfg(test)]` code couples test helpers to implementation details and clutters the source of truth.

## Decision

- The root `Cargo.toml` is a virtual manifest for a workspace. Members are `lib/*` and `bin/*`.
- Current members: `lib/agents` (`arcmis-agents`), `lib/tools` (`arcmis-tools`), `bin/harness` (`harness`, the main entry point). Shared package metadata and the clippy lint table live in `[workspace.package]` and `[workspace.lints]`.
- All tests reside in `{crate}/tests/`. Behavior tests use `cucumber-rs` 0.21 with `harness = false` runner binaries (`{crate}/tests/cucumber.rs`) and Gherkin files under `{crate}/tests/features/`.
- Each crate defines an empty `cucumber-tests` feature. The cucumber target sets `required-features = ["cucumber-tests"]`.

## Runner Contract

`cargo nextest` cannot list custom-harness test binaries. They do not speak the libtest protocol, so nextest's `--list` fails on them. The `required-features` gate keeps those targets out of the default workspace build. Consequences:

- Unit and listing flows stay on nextest: `cargo nextest run --profile ci --workspace --all-targets --no-tests=pass` sees zero custom binaries. See `.omp/rules/use-nextest.md`.
- Cucumber suites run explicitly: `cargo test --features cucumber-tests --workspace --test cucumber`.

## Consequences

- Adding a domain means a new `lib/<domain>` crate. Adding a binary means a new `bin/<name>` crate. The root manifest does not change.
- New integration tests go to `{crate}/tests/`. Feature files to `{crate}/tests/features/`. In-module test modules are a violation of this decision.
- `cargo test --features cucumber-tests ...` bypasses nextest. This is the one sanctioned exception to `.omp/rules/use-nextest.md`, forced by the libtest protocol gap.
