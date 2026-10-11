# 0010. User-facing code order, macro qualification, and module flattening

- **Date:** 2026-09-16
- **Status:** accepted. ADR 0011 supersedes the module-flattening point.
  ADR 0016 supersedes the macro-qualification point. Fully qualified paths
  and macros are out. Plain paths are in.

## Context

Tool files buried their tool under private helpers. The tool struct and its `Tool` impl sat at the bottom of the file, so a reader met `resolve` and `read` before the item the file exists for. `main.rs` declared `main` last. Function bodies mixed orchestration with detail. Tool descriptions embedded `Args: {...}` payloads that duplicated the `parameters()` schema. Two glue files existed only to re-list modules: `agent.rs` (`pub mod default;`) and `util.rs` (`pub mod path;`). Macro invocations came from bare imports (`format!`, `json!`) instead of qualified paths.

## Decision
- Fully qualified macros (`.omp/rules/rust.md` §9, superseded 2026-09-18 by
  ADR 0016: plain paths): every macro invocation outside `#[derive(...)]`
  attributes uses the defining crate path with a bang (`serde_json::json!`,
  `format!`). No `use` import to call a macro bare.
- Module flattening (`.omp/rules/layout.md` new §5): a file whose only content is module declarations must not exist. Declare `pub mod` inline in `lib.rs`. Shared helpers sit in their own source file (`src/path.rs`), not in a `util.rs` glue file.
- Enforcement: `lint-rules.py` fails unqualified macro invocations (comments and string literals excluded) and `Args:` payloads in description strings. Block order stays review-enforced.

## Consequences

- Every tool file now reads args, tool, impl, callees top-down. `main.rs` opens with `main`, then `init_tracing`, `load_config`, `discover_sources`, `run_attempts`, then the hook machinery.
- The three tool descriptions are single pure sentences. Model-facing wording changes no longer touch parameter schemas.
- Macro call sites carry the crate path. Removing a macro import cannot break a call site.
- `agents/src/default.rs` and `tools/src/path.rs` sit at the crate root. The `agent/` and `util/` directories are gone.
