# ARCMiS tools crate — shared contract for the 17 oh-my-pi-style tools

## Goal
Replace the current three-tool surface of `ARCMiS/lib/tools` with the 17-tool surface of oh-my-pi (read, write, edit, ast_grep, ast_edit, search, find, bash, eval, ssh, lsp, debug, task, irc, todo, job, ask). One Rust file per tool under `ARCMiS/lib/tools/src/`, each implementing `::rig::tool::Tool`.

## Hard constraints (violate = rewrite)
1. **Skip gates/formatters entirely.** Do NOT run cargo, clippy, fmt, nextest, or any lint script. Do not commit. Edit only. The orchestrator runs all gates.
2. **File layout per `.omp/rules/layout.md`**: one file per tool, `src/<tool_name>.rs`. NO `mod.rs`. Do not touch `lib.rs` — the orchestrator wires it.
3. **Block order (mechanically linted, code-clarity.md §Declaration before implementation + §Block order):**
   primary struct decl → `impl ::rig::tool::Tool for X` → args struct + other secondary structs → private helper fns in call order. Example:
   ```rust
   //! `write` writes one file.

   pub struct Write { ... }                      // primary

   impl ::rig::tool::Tool for Write { ... }      // impl directly after

   #[derive(...)]
   pub struct WriteArgs { ... }                  // secondary types after impl

   fn helper(...) -> ... { ... }                 // helpers in call order
   ```
   The lint fails on ANY `impl X` where `pub struct X` appears LATER in the file.
4. **Rust style (mechanically linted, `.omp/rules/rust.md`):**
   - Every external `use` is fully qualified: `use ::serde::Deserialize;` NEVER `use serde::...`. Prefer inline qualified paths over `use` where a symbol appears once or twice.
   - Every `#[derive(...)]` path qualified: `#[derive(::core::fmt::Debug, ::serde::Deserialize)]`.
   - Every macro qualified with bang: `::serde_json::json!(...)`, `::std::format!(...)`, `::std::vec![...]`.
   - No single-letter bindings except `i`/`j`/`k` in loops. Name everything (`error`, `output`, `root`).
   - No `println!`/`eprintln!`/`dbg!` — log via `::tracing::` when needed.
5. **STE English** (linted): comments and description strings use Simplified Technical English. No semicolons inside doc comments or `description()` strings (the STE linter flags them). Short active sentences, ≤ 25 words.
6. **Description purity:** `description()` returns ONE short sentence stating what the tool does. No `Args:` payloads, no JSON examples. `parameters()` carries the schema.
7. **Error style:** return `::rig::tool::ToolExecutionError` via `::rig::tool::ToolExecutionError::other(...)`, `::not_found(...)`, `::invalid_args(...)`, `::timeout(...)`, `::permission_denied(...)`. Never panic in `call`.
8. **Output:** `::rig::tool::ToolOutput::text(...)` for prose/markup results; `::json(...)` for structured data (bash, lsp, debug, job return JSON).
9. **Args types:** one `pub struct <Name>Args` per tool, `#[derive(::core::fmt::Debug, ::serde::Deserialize)]`, field types `::std::string::String`, `Option<...>`, `Vec<...>`, `bool`, numbers. Keep the oh-my-pi field names exactly (`path`, `content`, `input`, `pattern`, `paths`, `command`, `code`, `language`, `action`, `op`, `questions`, ...).

## Shared state pattern (stateful tools)
Tools needing cross-call state (bash session, eval kernel, todo list, job registry, file snapshot cache) take a `::std::sync::Arc<...State>` field on the tool struct. Declare the state type in the tool's own file if only that tool uses it; the shared snapshot store is in `src/util/snapshots.rs`, the job registry in `src/util/jobs.rs`, both owned by the coordination worker — other workers MUST NOT create those two files, they reference the types below.

```rust
// src/util/snapshots.rs (provided by coordination worker)
pub struct SnapshotStore { ... }   // mint/lookup ¶PATH#TAG tags + line snapshots
impl SnapshotStore {
    pub fn new() -> ::std::sync::Arc<Self> { ... }
    pub fn mint(&self, path: &str, text: &str) -> String { ... }        // returns 4-hex TAG
    pub fn lookup(&self, tag: &str) -> Option<(String, Vec<String>)> { ... } // (path, lines)
    pub fn lookup_by_path(&self, path: &str) -> Option<(String, Vec<String>)> { ... }
}
```
If `snapshots.rs`/`jobs.rs` do not exist when you build your tool, declare your state as a plain field the orchestrator can construct later, and note it in your report. Do NOT create those two files if another worker owns them (see assignment).

## Tool instance shape
Every tool struct is a plain value with config fields (root dirs, timeouts, shared `Arc` state) — exactly like the existing `ReadFile { root }` pattern. No builder, no trait objects. The orchestrator constructs them in `lib.rs`/the agent.

## Existing files you may reference (do not modify)
- `ARCMiS/lib/tools/src/read_file.rs`, `write_file.rs`, `run_command.rs` — the Tool impl pattern to copy.
- `ARCMiS/lib/tools/src/util/path.rs` — `path_sanitize(root, path)` for path sandboxing.
- Cargo deps available: rig 0.42, tokio (fs, io-util, process, macros, rt-multi-thread, sync), regex, globset, walkdir, ignore, grep-searcher, grep-regex, ast-grep-core, rusqlite, uuid, sha2, libc, serde/serde_json/serde_yaml, tracing.

## Report format
End with: files created, each public symbol (tool struct + Args struct), and any shared-state type you defined that others must use.
