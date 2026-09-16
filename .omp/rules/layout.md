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

Put shared helpers in their own source file, for example `src/path.rs`. Never make a `util.rs` dumping ground. Never put helpers next to the item list.

## 5. Inline Module Declarations

A file whose only content is module declarations must not exist. Do not keep a glue file that only re-lists modules with `pub mod foo;`. Declare `pub mod foo;` inline in `lib.rs` and delete the glue file.

- Keep a `src/foo/` directory for real multi-file submodules. A directory is fine when its files hold code.


## 6. Examples

Current tree:

- agents crate: `src/default.rs` (agent), `src/lib.rs` holds the module list.
- tools crate: `src/read_file.rs`, `src/write_file.rs`, `src/run_command.rs`, `src/catalog.rs` (tools), `src/path.rs` (helpers, declared inline in `lib.rs`).


