# CH-Trie C → Rust Translation Plan

*Rewritten after inspecting the real C source (`src/chtrie.c`, `src/chtrie.h`,
`tests/test.c`, `Makefile`). The previous plan was drafted before the source
was restored and contained two factual errors, both corrected here:*

1. **The edge hash table is global, not per-node.** `chtrie` has ONE flat
   table `etab` of `ecap` buckets; each bucket is a singly linked list of
   edges `{from, sym, to}`. The key is the pair `(from, sym)` hashed by
   `h = (from * alphsz + sym) % ecap`. (Old plan wrongly said "per-node
   edge hash table".)
2. **`ERANGE` is an alloc-time size check, not a symbol-range error.**
   `chtrie_alloc` sets `errno = ERANGE` when `n > INT_MAX`, `m > INT_MAX`,
   or when `INT_MAX - (n-1) < (n-1)/3` (overflow guard for
   `ecap = (n-1) + (n-1)/3`). `ENOMEM` is set by `chtrie_walk` when the
   node-index pool is exhausted (`idxptr == idxpool && idxmax >= maxn`).
   (Old plan wrongly mapped `ERANGE` to "symbol does not fit `m` bits".)

## 1. What the C source actually does

- `chtrie_alloc(size_t n, size_t m) -> chtrie *`
  - Clamps `n, m` to ≥ 1; returns `NULL` + `errno = ERANGE` if `n` or `m`
    exceeds `INT_MAX` or the `ecap` computation would overflow.
  - `ecap = (n-1) + (n-1)/3`; allocates `etab` (ecap pointers),
    `idxpool` (n ints). Root node is index 0; `idxmax = 1`;
    `idxptr = idxpool` (empty free list).
- `chtrie_walk(tr, from, sym, creat) -> int`
  - Hashes `(from, sym)` into the global table, scans the bucket's linked
    list; returns child index on hit, else `-1`.
  - With `creat`: if the pool is empty and `idxmax >= maxn`, sets
    `errno = ENOMEM` and returns `-1`; otherwise allocates an edge,
    prepends it to the bucket, and takes the child index from the free
    list (`*--idxptr`) or from the growing counter (`idxmax++`).
  - Note: a plain `malloc` failure also returns `-1` *without* setting
    errno (C quirk; in Rust, allocation failure aborts or we map to the
    same error variant).
- `chtrie_del(tr, from, sym) -> void`
  - Finds the edge in the bucket, unlinks it, frees it, and recycles the
    child index via `*idxptr++ = p->to`. No-op if absent.
- `chtrie_free(tr) -> void` — frees all edges, `etab`, `idxpool`, the trie.
- `tests/test.c` — builds a word set with `add`/`del`/`query` helpers over
  a trie of `N = 65536` nodes / `M = 256` symbols; uses static arrays
  `term[N]`, `nchild[N]` (and `nodes[N]`, `symbs[N]` inside `del`) to track
  termination and child counts; runs 14 query assertions.

## 2. Strategy

### Crate layout

```
chtrie/
├── Cargo.toml          # name = "chtrie", edition 2021, no dependencies
├── src/
│   └── lib.rs          # ChTrie, ChTrieError, Edge, arena + free list, Drop
└── tests/
    └── test.rs         # 1:1 port of tests/test.c (helpers + 14 assertions)
```

Single crate, single module — the library is 4 functions. No `unsafe`
target; if any construct forces it, isolate and document it.

### C API → Rust mapping

| C | Rust | Notes |
|---|------|-------|
| `chtrie *chtrie_alloc(size_t n, size_t m)` | `ChTrie::alloc(n: usize, m: usize) -> Result<ChTrie, ChTrieError>` | clamps to ≥ 1; `Err(ChTrieError::Range)` on the ERANGE conditions |
| `int chtrie_walk(chtrie *, int from, int sym, int creat)` | `ChTrie::walk(&mut self, from: i32, sym: i32, creat: bool) -> Result<Option<i32>, ChTrieError>` | `Ok(Some(child))` = found/created; `Ok(None)` = not found (C's `-1` without errno); `Err(ChTrieError::Capacity)` = C's `-1` + `ENOMEM` (pool exhausted) |
| `void chtrie_del(chtrie *, int from, int sym)` | `ChTrie::del(&mut self, from: i32, sym: i32)` | no-op if absent; recycles child index |
| `void chtrie_free(chtrie *)` | `impl Drop for ChTrie` | RAII; no public `free` |
| `errno == ERANGE` | `ChTrieError::Range` | alloc-time size/overflow guard |
| `errno == ENOMEM` | `ChTrieError::Capacity` | node-index pool exhausted |

Root node index is `0`; expose `pub const ROOT: i32 = 0` for helpers/tests.

### Error handling

`enum ChTrieError { Range, Capacity }` replaces NULL + errno. `walk`
distinguishes "not found" (a normal outcome → `Ok(None)`) from "cannot
create" (an error → `Err(Capacity)`), which is exactly the C contract
(`-1` with vs. without `errno`).

### Memory management

- `ChTrie` owns everything; `Drop` replaces `chtrie_free`.
- **Edge table:** `etab: Vec<Option<Edge>>` of length `ecap`, where
  `Edge { from: i32, sym: i32, to: i32 }`. The C linked list per bucket
  becomes a `Vec<Edge>` bucket (or `Vec<Option<Edge>>` with `next` indices
  into an edge arena — equivalent; prefer the simple per-bucket `Vec`).
  Hash and bucket order preserved so behavior matches.
- **Index pool:** `idxpool: Vec<i32>` (capacity `n`) as the free list,
  `idxptr: usize` (number of recycled indices), `idxmax: i32` (next fresh
  index). `walk(creat)` pops from the pool or increments `idxmax`;
  `del` pushes back. This is a free-list, not a node arena — the C code
  stores *no per-node data at all*; nodes are just indices. Keep it that
  way (do not invent a `Vec<Node>`).
- No `Box`, no raw pointers, no `unsafe`; `Send`/`Sync` fall out naturally.

### Tests

- `tests/test.c` → `tests/test.rs` as an integration test:
  - Same data: `dict1 = ["", "the", "a", "an"]`, `dict2 = ["he", "she",
    "his", "hers"]`, `dict3 = ["this", "that"]`, `stop = ["the", "an",
    "a"]`; `N = 65536`, `M = 256`.
  - Same helpers `add`, `del`, `query` (string → per-byte `walk`/`del`
    calls; `term` and `nchild` as `Vec<i32>` of size `N`; `del`'s path
    trace `nodes`/`symbs` as local `Vec`s).
  - Same 14 `(test_case, expected)` assertions, via `assert_eq!` — none
    weakened or deleted.
- Extra edge-case tests (ERANGE on huge `n`/`m`, pool exhaustion →
  `Err(Capacity)`, index recycling after `del`) go in a `#[cfg(test)]`
  module in `lib.rs`, keeping `tests/test.rs` a faithful 1:1 port.

### Makefile → Cargo.toml

| Makefile | Cargo |
|----------|-------|
| `make all` (compile `chtrie.o`) | `cargo build` |
| `make test` (link + run `run.tmp`) | `cargo test` |
| `make install` (header + static lib) | `cargo package` / `cargo install` (not needed for this task) |
| `make clean` | `cargo clean` |

`Cargo.toml`: `name = "chtrie"`, `edition = "2021"`, no dependencies.

## 3. Seed tasks

1. **T1 — Scaffold crate:** `Cargo.toml` + `src/lib.rs` stub; `cargo build`
   passes.
2. **T2 — Implement `ChTrie::alloc`:** clamping, ERANGE/overflow checks,
   `ecap`, edge table, index pool; `ChTrieError { Range, Capacity }`;
   `Drop`.
3. **T3 — Implement `walk` and `del`:** global bucket lookup, create with
   pool exhaustion → `Err(Capacity)`, free-list pop/push recycling.
4. **T4 — Port `tests/test.c` → `tests/test.rs`:** `add`/`del`/`query`
   helpers with `term`/`nchild` state and all 14 assertions unchanged.
5. **T5 — Unit tests in `lib.rs`:** ERANGE on oversized `n`/`m`, pool
   exhaustion, index recycling (del then re-create reuses the index).
6. **T6 — Final gate:** `cargo build` clean, `cargo test` green,
   `cargo clippy` clean; update `notes.md`.
