# 0009. Crate naming, manifest structure, and module layout

- **Date:** 2026-09-16
- **Status:** accepted

## Context

Workspace crates were `arcmis-agents` and `arcmis-tools`. Rust code imported them as `::arcmis_agents::...`, which is verbose and repeats the repo name. Members declared their own dependency versions (`rig = "0.42"`), so version drift across crates was likely. Manifests sorted dependencies arbitrarily and pinned versions with two components. One module still used the `mod.rs` style, and a shared helper sat inside a tool file.

Cargo mechanics, verified by compiling a scratch workspace:
- A package named `ARCMiS-foo` builds. Cargo emits a `non_snake_case` warning for the crate name, which CI tolerates (no `-D warnings`).
- A dependency alias `foo = { package = "ARCMiS-foo", ... }` makes Rust code import `::foo::`.
- A self-referential dev-dependency does not resolve. A `[lib] name = "foo"` override makes the crate's own integration tests import `::foo`.

## Decision

- Three new rules. `.omp/rules/naming.md`: workspace packages are `ARCMiS-<name>`, dependencies alias to the bare name, all Rust imports go through the alias. `.omp/rules/manifest.md`: dependency order in three groups, `x.y.z` pins, workspace inheritance, inline single-attribute form. `.omp/rules/layout.md`: no `mod.rs`, one item per file, `src/<group>/<tool>.rs` for tool groups, helpers in `src/util/<topic>.rs`.
- The crates rename to `ARCMiS-agents` and `ARCMiS-tools`. Binaries keep plain names (`harness`). Each lib crate sets `[lib] name` to the bare alias name. Shared dependencies (rig, serde, serde_json, serde_yaml, tokio, tracing, tracing-subscriber, bon, cucumber) live in `[workspace.dependencies]` and members inherit.
- `path_sanitize` moves from `write_file.rs` to `src/util/path.rs`. The `agent/mod.rs` module list moves to `agent.rs`.
- `lint-rules.py` gains: lowercase `arcmis-` package names, `arcmis_` imports, `mod.rs` under `ARCMiS/`, and an advisory for two-component pins. Manifest dependency order stays review-enforced.
- The `non_snake_case` warning from the crate names is deliberate. Cargo emits it. CI does not deny warnings. The warning documents the naming choice, so the tree keeps it.

## Consequences

- Imports read `::agents::Config` and `::tools::Catalog` everywhere, self tests included. Adding a lib crate means one `[lib] name` line plus the alias at consumers.
- Version bumps happen once, in the root manifest. Feature additions stay inline at the member.
- `cucumber` stays at 0.21.1 while 0.23.0 exists: ADR 0006 pins the cucumber-tests feature shape and the `harness = false` runner config at 0.21. Bumping it is a separate migration.
