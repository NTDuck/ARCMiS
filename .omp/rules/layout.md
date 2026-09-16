---
description: No mod.rs. One item per file. Shared helpers live in src/util/<topic>.rs.
---

# Module Layout

## 1. No mod.rs

Do not create `mod.rs` files anywhere under `ARCMiS/`. Use the modern module layout: `src/foo.rs` holds the code of module `foo`. A directory `src/foo/` holds the submodules of `foo` when `foo` grows beyond one file. See `.omp/rules/README.md` for the example tree.

## 2. One Item per File

Each crate has one source file per item.

- One agent per file: `src/agent/<agent-name>.rs`.
- One tool per file: `src/<tool-name>.rs`.

## 3. Tool Groups

When several tools form one group, use `src/<group>/<tool>.rs`.

## 4. Shared Helpers

Put shared helpers in `src/util/<topic>.rs`. Never make a `util.rs` dumping ground. Never put helpers next to the item list.

## 5. Examples

Current tree:

- agents crate: `src/agent/default.rs` (agent), `src/agent.rs` (module list), `src/util/…` (helpers).
- tools crate: `src/read_file.rs`, `src/write_file.rs`, `src/run_command.rs`, `src/catalog.rs`, `src/util/…` (helpers).
