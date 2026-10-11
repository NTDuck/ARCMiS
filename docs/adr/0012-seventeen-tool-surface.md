# 0012. Seventeen-tool surface after oh-my-pi

- **Date:** 2026-09-17
- **Status:** accepted

## Context

The tools crate exposed three tools: `read_file`, `write_file`, and `run_command`. The migration agent needs the surface of oh-my-pi (https://github.com/dankalish/oh-my-pi): seventeen tools across files and search, runtime, code intelligence, and coordination. The old three-tool set cannot express exploration, structural search, or coordination, so the model degrades to shell-only workarounds.

## Decision

- Adopt the seventeen-tool surface: `read`, `write`, `edit`, `search`, `find`, `ast_grep`, `ast_edit`, `bash`, `eval`, `ssh`, `lsp`, `debug`, `task`, `irc`, `todo`, `job`, `ask`. One module per tool under `src/`, each implementing `::rig::tool::Tool` with the oh-my-pi argument names.
- Shared state lives in `src/util/`: `SnapshotStore` mints the `¶PATH#TAG` hashline anchors that `read`, `write`, `edit`, and `search` exchange, and `JobRegistry` tracks background work for `task` and `job`. Stateful tools hold their state behind `::std::sync::Arc` fields that the host constructs.
- The legacy `read_file`, `write_file`, and `run_command` modules are gone. The new `Write` and `Bash` tools take their places.
- The two ast tools use `ast-grep-language` (concrete `SupportLang` values) on top of `ast-grep-core`. `ast-grep-core` alone ships the `Language` trait, not concrete languages.

## Consequences

- Adding a tool means one module, one `pub mod` line, one `pub use`, and one contract-conformant file.
- `rusqlite` uses the `bundled` feature so test binaries link without a system sqlite3.
- The tokio feature set grows `fs`, `io-util`, `sync`, and `time`. The process watchdog uses `tokio::time::timeout` with a child kill on expiry.
- `SnapshotStore` uses interior `::std::sync::Mutex` maps because rig tools must be `Send` and `Sync`. A `RefCell` interior breaks the tool trait bounds.
