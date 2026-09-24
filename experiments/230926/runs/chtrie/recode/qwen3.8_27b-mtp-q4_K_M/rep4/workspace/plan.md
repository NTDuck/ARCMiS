# CH-Trie → Rust Translation Plan

Source: `assets/ReCodeAgent/data/tool_projects/crust/chtrie/c` (C89, BSD-3-Clause).
Target: standalone Rust crate in this workspace, `cargo test` as the acceptance gate.

## 1. Fragment extraction (source symbols)

`src/chtrie.h`
- `struct chtrie_edge` — `{ struct chtrie_edge *next; int from, sym, to; }`
- `struct chtrie` — `{ etab; idxpool, idxptr, idxmax; maxn, alphsz, ecap; }`
- `chtrie_alloc(size_t n, size_t m)`
- `chtrie_walk(chtrie*, int from, int sym, int creat)`
- `chtrie_del(chtrie*, int from, int sym)`
- `chtrie_free(chtrie*)`

`src/chtrie.c`
- `chtrie_alloc` — clamp n/m to ≥1; `ERANGE` if `n>INT_MAX`/`m>INT_MAX`/`ecap` overflow;
  `calloc` struct, `etab` (size `ecap`), `idxpool` (size `n`); `idxmax=1`, `idxptr=idxpool`.
- `chtrie_walk` — hash `h=(from*alphsz+sym)%ecap`; scan bucket chain; on `creat`
  allocate edge, push to head, recycle `*--idxptr` or take `idxmax++`;
  `ENOMEM` when pool empty and `idxmax>=maxn`.
- `chtrie_del` — find edge in bucket, unlink, push `p->to` onto pool, `free(p)`; no-op if absent.
- `chtrie_free` — free all edges, `etab`, `idxpool`, struct.

`tests/test.c`
- `dict1`, `dict2`, `dict3`, `stop` — string sets.
- `term[N]`, `nchild[N]` — per-node termination / child-count arrays.
- `add(char*)`, `del(char*)`, `query(char*)` — trie-level helpers.
- `main` — build set, run 14 query assertions
  (expected `0,0,1,1,1,0,1,0,0,1,1,0,1,1`).

Not ported: `test.c_old`, `test.in`, `test.out` (legacy interactive fixtures, not in the
`cargo test` acceptance set).

## 2. Name mapping (C → Rust)

| C symbol | Rust symbol | Note |
|---|---|---|
| `struct chtrie_edge` | `Edge` | `next` pointer → `Option<usize>` index into `edges` pool |
| `struct chtrie` | `ChTrie` | fields renamed to `snake_case` |
| `chtrie_alloc` | `ChTrie::new` | `size_t` → `u32`; `NULL`+`errno` → `Result<_, ChTrieError>` |
| `chtrie_walk(…,creat=0)` | `ChTrie::walk` | returns `Option<u32>` (miss is not an error) |
| `chtrie_walk(…,creat=1)` | `ChTrie::walk_or_create` | returns `Result<u32, ChTrieError>` |
| `chtrie_del` | `ChTrie::del` | no-op if absent |
| `chtrie_free` | `Drop` (implicit) | `Vec`s free themselves; no manual impl |
| `errno` `ERANGE` | `ChTrieError::Range` | |
| `errno` `ENOMEM` | `ChTrieError::Capacity` | |
| `int` node/sym indices | `u32` | `INT_MAX` bound → `u32::MAX` |
| `tests/test.c` `main` | `tests/test.rs` `test_suite` | `assert` → `assert_eq!` in `#[test]` |
| `add`/`del`/`query` | `add`/`del`/`query` (free fns) | take `&mut ChTrie` + `term`/`nchild` slices |

## 3. Skeleton (already written)

- `Cargo.toml` — package `chtrie`, edition 2021, no deps, standalone `[workspace]`.
- `src/lib.rs` — `Edge`, `ChTrieError` (+`Display`/`Error`), `ChTrie` with
  `new`/`walk`/`walk_or_create`/`del` stubs, plus 5 unit-test stubs.
- `tests/test.rs` — `add`/`del`/`query` helper stubs + `test_suite` stub.

All stubs compile (`cargo build` succeeds; only unused-variable warnings).

## 4. Implementation plan

### Part A — source files (bottom-up dependency order)
1. `src/lib.rs`
   - `ChTrieError::{Range, Capacity}` + `Display` + `Error`.
   - `Edge` struct.
   - `ChTrie::new` — clamp, `ERANGE` guards (`checked` math on `ecap`), init `etab`/`edges`/`idxpool`.
   - `ChTrie::walk` — hash in `u64`, scan bucket chain, return `Option<u32>`.
   - `ChTrie::walk_or_create` — reuse `walk`; on miss allocate edge, recycle/allocate index, `Capacity` on exhaustion.
   - `ChTrie::del` — find + unlink edge, recycle `to` index; no-op if absent.
   - Unit tests: `alloc_clamps_small_values`, `alloc_rejects_huge_n`,
     `walk_miss_and_create`, `del_absent_is_noop`, `index_recycled_after_del`.

### Part B — test files (bottom-up dependency order)
1. `tests/test.rs`
   - `add`/`del`/`query` helpers over `u8` symbols using `walk`/`walk_or_create`/`del`.
   - `test_suite` — build the dict/stop-word set, run the 14 query assertions
     (`0,0,1,1,1,0,1,0,0,1,1,0,1,1`) with `assert_eq!`.

### Acceptance
`cargo test` passes: 5 unit tests + 1 integration test (14 assertions).
