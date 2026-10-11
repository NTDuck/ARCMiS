# 0011. Util directory layout

- **Date:** 2026-09-17
- **Status:** accepted

## Context

ADR 0010 flattened the helper modules. `tools/src/path.rs` sat at the crate root next to the tool files. The agents crate kept `config.rs`, `measure.rs`, `registry.rs`, and `sources.rs` at the root next to `default.rs`. The crate root mixed domain items with support code, and a reader could not tell tool from helper by location. ADR 0009 had already prescribed helpers in `src/util/<topic>.rs`. The flattening step dropped that grouping.

## Decision

- Support modules live in `src/util/`, one topic per file. Agents crate: `util/{config,measure,registry,sources}.rs`. Tools crate: `util/{catalog,path}.rs`. The item files (`default.rs`, `read_file.rs`, `write_file.rs`, `run_command.rs`) stay at the crate root.
- `lib.rs` declares the group as one inline `pub mod util { ... }` block. A bare `pub mod util::foo;` does not parse (verified on rustc 1.97). A separate `util.rs` declaration file would violate `.omp/rules/layout.md` §5. `.omp/rules/layout.md` §4 now states the `src/util/` rule. §5 states the inline-block mechanics.
- Root re-exports (`pub use util::config::Config`, `pub use util::path::path_sanitize`, and so on) keep the public API paths unchanged for crate items.

## Consequences
- The crate root lists only domain items. Support code sits one `util::` step away.
- New support modules go to `src/util/<topic>.rs` plus one line in the inline block. The block stays declaration-only.
- ADR 0010 stated "shared helpers sit at `src/path.rs`, not in a `util.rs` glue file" and recorded "the `util/` directories are gone". This decision supersedes both statements on the layout point only. ADR 0010 keeps its block-order and macro-qualification decisions.
