---
description: No mod.rs. No module-declaration glue files. One item per file. Shared helpers live in their own source file.
---


## 1. No mod.rs

Do not create `mod.rs` files anywhere under `ARCMiS/`. Use the modern module layout: `src/foo.rs` holds the code of module `foo`. A directory `src/foo/` holds the submodules of `foo` when `foo` grows beyond one file. See `.omp/rules/README.md` for the example tree.

## 2. One Item per File

Each crate has one source file per item.

- One agent per file: `src/agent/<agent-name>.rs`.
- One tool per file: `src/<tool-name>.rs`.

## 3. Tool Groups

When several tools form one group, use `src/<group>/<tool>.rs`.

## 4. Shared Helpers

Put shared helpers in their own source file under `src/util/`, for example `src/util/path.rs`. One topic per file. Never make a `util.rs` dumping ground. Never put helpers next to the item list.

## 5. Inline Module Declarations

A file whose only content is module declarations must not exist. Do not keep a glue file that only re-lists modules with `pub mod foo;`. Declare `pub mod foo;` inline in `lib.rs` and delete the glue file.

- Keep a `src/foo/` directory for real multi-file submodules. A directory is fine when its files hold code.
- A module group such as `util` needs one declaration in `lib.rs`. A bare `pub mod util::foo;` does not parse, so declare the group as one inline block (`pub mod util { pub mod foo; }`). The block holds declarations and at most a doc comment. It is not a glue file.


## 6. Examples

Current tree:

- agents crate: `src/default.rs` (agent), `src/util/` (config, measure, registry, sources), `src/lib.rs` declares `util` inline as one `pub mod util { ... }` block.
- tools crate: the seventeen tool modules at the crate root. Names: `read.rs`, `write.rs`, `edit.rs`, `search.rs`, `find.rs`, `ast_grep.rs`, `ast_edit.rs`, `bash.rs`, `eval.rs`, `ssh.rs`, `lsp.rs`, `debug.rs`, `task.rs`, `irc.rs`, `todo.rs`, `job.rs`, `ask.rs`. Then `src/util/` (catalog, jobs, path, snapshots). Then `src/lib.rs`, which declares `util` inline and re-exports the tools.

