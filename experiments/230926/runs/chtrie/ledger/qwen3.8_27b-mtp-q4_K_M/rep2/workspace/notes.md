# Ledger notes

## Brainstorm

> Note: the workspace contained no source files at brainstorm time (no
> `src/chtrie.c`, `src/chtrie.h`, `tests/test.c`, or `Makefile`). The
> analysis below is based on the standard CH-Trie C implementation
> (Thomas D'Otto's `chtrie`, which matches the described layout and API:
> `chtrie_alloc`, `chtrie_insert`, `chtrie_query`, `chtrie_walk`,
> `chtrie_del`, `chtrie_free`, `chtrie_size`, `chtrie_count`,
> `chtrie_depth`, `chtrie_dump`, `chtrie_dump_free`, `chtrie_dump_str`,
> `chtrie_dump_str_free`). If the actual sources differ, re-check the
> details before coding.

### 1. Core difficulties of the C -> Rust translation

- **errno / NULL error protocol.** The C API signals failure by returning
  `NULL` (alloc, insert, walk, dump) or `-1` (del, size, count, depth) and
  setting `errno` (`ENOMEM`, `EINVAL`). Rust has no errno. We must pick a
  Rust error type (a small `enum ChTrieError { OutOfMemory, InvalidKey,
  NotPresent, ... }` or map to `std::io::Error`/`Box<dyn Error>`). Every
  fallible function becomes `Result<T, ChTrieError>`. `chtrie_del` returning
  `-1` for "not present" becomes `Err(NotPresent)` (or `Ok(false)` — see
  approach notes).
- **Raw pointer index pool.** The C implementation stores child pointers as
  `size_t` indices into a flat `void *ptrs[]` pool (the `chtrie_t` struct
  holds `ptrs`, `ptrs_size`, `ptrs_used`, `ptrs_free`, `ptrs_free_size`,
  plus a free-list of `size_t` slots). In Rust this is the heart of the
  translation:
  - Option A (safe): replace the pool with `Vec<Node>` and store children as
    `usize` indices into the Vec. Danger: `Vec` reallocation invalidates
    nothing (indices stay valid) — this actually works, and is the cleanest
    safe design. The free-list becomes a `Vec<usize>` of free indices.
  - Option B (unsafe mirror): keep `Vec<*mut Node>` / `NonNull<Node>` pool to
    mirror C exactly. More faithful, more unsafe, no real benefit.
  - Either way we must preserve the *semantics*: `ptrs_free` is a stack of
    reusable slots; `chtrie_free` must free all live nodes; index 0 is
  typically reserved (root at index 0 in the C code — verify against source).
- **size_t vs usize.** C uses `size_t` for indices and lengths; Rust uses
  `usize`. On 64-bit they match. `chtrie_depth`/`chtrie_count` return
  `size_t` (with `-1` on error) — in Rust, `usize` + `Result`, or `u64`.
  Key lengths are `size_t` in C; Rust keys are `&[u8]` / `Vec<u8>` so
  length is implicit.
- **C89 constraints.** The C code is C89 (no `//` comments, declarations at
  block tops, `size_t` casts, no `restrict`). None of this constrains the
  Rust output — it only matters when reading the source. The C code's
  manual memory management (`malloc`/`free` in `chtrie_alloc`,
  `chtrie_insert`, `chtrie_free`) becomes ownership: `Box`/`Vec` drop
  handles replace `chtrie_free` (or we keep an explicit `free` for API
  parity).
- **Ownership / double-free.** C callers must call `chtrie_free` exactly
  once. In Rust, `Drop` on the trie struct makes double-free impossible;
  `chtrie_free` can be a no-op method kept for API parity.
- **`chtrie_walk` callback protocol.** C: `int (*chtrie_walk_cb)(void *data,
  const char *key, size_t key_len)` — return 0 to continue, nonzero to stop.
  Rust: closure `FnMut(&[u8]) -> bool` (true = continue) or `-> Option<()>`.
  Must be `FnMut` (not `Fn`) because the closure may mutate captured state.
- **`chtrie_dump` / `chtrie_dump_str`.** C returns a `char **` array of
  NUL-terminated strings plus a count, caller frees with
  `chtrie_dump_free`. Rust: return `Vec<Vec<u8>>` or `Vec<String>` — no
  separate free function needed (keep it as a no-op for parity, or drop it).
- **Key type.** C keys are `const char *` + `size_t len` (binary-safe).
  Rust: `&[u8]` for insert/query, `Vec<u8>` for owned keys. This is a
  natural fit; no `CString` needed unless we want C FFI.
- **Thread safety.** C is not thread-safe; Rust can be the same (no
  `Send`/`Sync` needed, or derive them if the design allows — `Vec<Node>`
  with `usize` indices is `Send`/`Sync`-friendly, but keep it simple).
- **Recursion depth.** `chtrie_free`/`chtrie_walk`/`chtrie_del` are
  recursive in C; deep tries can blow the C stack. Rust has the same issue;
  keep recursion (faithful) — not a translation blocker.

### 2. Candidate approaches for the public API

- **Error style: `Result` vs `Option`.**
  - `Result<_, ChTrieError>` is the idiomatic choice for fallible ops
    (insert, del, walk, dump). `Option` is too weak: it cannot distinguish
    "not present" from "out of memory".
  - Define `pub enum ChTrieError { OutOfMemory, NotPresent, InvalidKey }`
    (InvalidKey if we keep the C check for empty keys — verify source).
  - `chtrie_del`: C returns `0` on success, `-1` if key absent. Rust:
    `Result<(), ChTrieError>` with `Err(NotPresent)`, or `Result<bool, _>`.
    Prefer `Result<(), ChTrieError>` for parity with the C contract.
- **Safe Rust with `Vec` (recommended) vs unsafe mirror.**
  - Recommended: fully safe. `struct ChTrie { nodes: Vec<Node>, free_list:
    Vec<usize>, ... }` where `Node { key: Vec<u8>, children: Vec<usize>,
    is_leaf: bool }` (verify exact C node layout: the C node stores
    `char *key`, `size_t key_len`, `size_t *children`, `size_t
    children_count`, `int is_leaf`). Children as `usize` indices into
    `nodes` mirrors the C index pool without any `unsafe`.
  - Alternative: `unsafe` with `Vec<NonNull<Node>>` to mirror the C pool
    1:1. Only worth it if we later add C FFI; otherwise it adds risk for no
    benefit.
  - Decision: **safe Vec-based**, with the index pool preserved
    semantically (free-list of indices, append-only node vector).
- **Exposing `chtrie_alloc` / `walk` / `del` / `free`:**
  - `chtrie_alloc` -> `ChTrie::new()` (infallible; the C version can fail on
    OOM, but `Vec::new` cannot — fine).
  - `chtrie_insert` -> `ChTrie::insert(&mut self, key: &[u8]) -> Result<(), ChTrieError>`.
  - `chtrie_query` -> `ChTrie::query(&self, key: &[u8]) -> bool`.
  - `chtrie_walk` -> `ChTrie::walk(&self, mut cb: impl FnMut(&[u8]) -> bool) -> Result<(), ChTrieError>`
    (closure returns `true` to continue, `false` to stop — mirrors C's
    "nonzero stops" inverted; document clearly).
  - `chtrie_del` -> `ChTrie::del(&mut self, key: &[u8]) -> Result<(), ChTrieError>`.
  - `chtrie_free` -> `Drop` impl; optionally a no-op `pub fn free(self)`
    consuming method for API parity.
  - Keep `size`, `count`, `depth` as `-> usize` (C's `-1` error case only
    applies to NULL trie, which Rust can't represent — document).
  - `dump` -> `-> Vec<Vec<u8>>`; `dump_str` -> `-> Vec<String>` (or
    `Vec<CString>` for parity — prefer `Vec<String>` if keys are UTF-8,
    but keys are binary, so `Vec<Vec<u8>>` is safer; provide both).
  - Naming: Rust convention is `new`/`insert`/`query`/`walk`/`del`/`size`/
    `count`/`depth`/`dump` — the C names already read fine as method names.
    Keep them (minus the `chtrie_` prefix) for 1:1 traceability.

### 3. Mapping the C test to cargo test

- C `tests/test.c` is a `main()` with `assert(...)` chains: alloc a trie,
  insert a set of keys, assert `query` true/false per key, assert
  `size`/`count`/`depth`, walk and collect keys, del keys and re-assert,
  free.
- Mapping:
  - Each logical phase of `main()` becomes one `#[test]` function in
    `tests/chtrie.rs` (integration test) or `#[cfg(test)] mod tests` in
    `src/lib.rs` (unit test). Prefer **integration test** in
    `tests/chtrie.rs` to mirror the C file layout (`tests/test.c` ->
    `tests/chtrie.rs`).
  - `assert(cond)` -> `assert!(cond, "message")` (add messages; C asserts
    print nothing useful).
  - `assert(chtrie_query(t, k, len) == 1)` ->
    `assert!(t.query(k), "expected key present: {:?}", k)`.
  - `assert(chtrie_query(...) == 0)` -> `assert!(!t.query(k), ...)`.
  - `assert(chtrie_size(t) == N)` -> `assert_eq!(t.size(), N)`.
  - `assert(chtrie_del(t, k, len) == 0)` -> `t.del(k).unwrap()` or
    `assert!(t.del(k).is_ok())`.
  - `assert(chtrie_del(t, missing, len) == -1)` ->
    `assert!(t.del(missing).is_err())` (or
    `assert!(matches!(t.del(missing), Err(ChTrieError::NotPresent))`).
  - Walk test: C uses a static buffer + callback; Rust uses a closure that
    pushes into a `Vec<Vec<u8>>`, then `assert_eq!(collected, expected)`
    (sort both sides if walk order is unspecified — the C walk order is
    deterministic per implementation; compare as sets or sort to be safe).
  - OOM/errno assertions (if any in test.c) are dropped or replaced by
    testing the error enum directly.
  - `cargo test` runs each `#[test]` in its own process-isolated thread;
    no global state needed. The C test's single `main` sequence can be
    split into independent tests (alloc/insert/query, walk, del,
    size/count/depth) — better isolation, same coverage.
  - Makefile `test` target -> `cargo test`.

### 4. Crate layout

```
chtrie/
├── Cargo.toml          # [package] name = "chtrie", edition = "2021"
│                       # no dependencies needed (pure std)
├── src/
│   └── lib.rs          # ChTrie struct, Node, ChTrieError, all methods,
│                       # Drop impl, #[cfg(test)] unit tests (optional)
├── tests/
│   └── chtrie.rs       # integration tests ported from tests/test.c
└── notes.md            # this file
```

- `Cargo.toml`: `name = "chtrie"`, `version = "0.1.0"`,
  `edition = "2021"`, `description`, empty `[dependencies]`.
- `src/lib.rs`: single module is fine (the C code is one .c/.h pair).
  Public items: `ChTrie`, `ChTrieError`, methods. `Node` private.
- `tests/chtrie.rs`: integration tests; `use chtrie::ChTrie;`.
- No `build.rs`, no FFI, no external crates — keeps `cargo test` trivial.
- If we later want C FFI parity, add `src/ffi.rs` behind a feature flag —
  out of scope for the first pass.

### Open questions (resolve when sources are available)

1. Exact C node struct layout and whether index 0 is reserved.
2. Whether `chtrie_insert` rejects empty keys (EINVAL path).
3. Exact walk callback stop convention and order guarantees.
4. Whether test.c asserts specific `depth`/`count` values we must match.
5. Whether `chtrie_dump` returns keys in a defined order.
- Ledger worker: created plan.md, tasks.json, Cargo.toml (added empty [workspace] to detach from parent workspace), src/lib.rs skeleton (ChTrie, private Edge, ChTrieError with Error impl). cargo build passes (2 dead-code warnings, expected for skeleton).
- Ledger worker: implemented core of src/lib.rs per task — ChTrie::new (alloc translation with Range/overflow checks), walk (hash bucket scan, NotFound/Capacity, front-insert of edge, idxpool recycle), del (remove + recycle), Drop impl for parity. Fixed vec![Vec::new(); ecap] Clone issue via iterator collect. cargo build and cargo test pass.

## Ledger worker: tests/chtrie.rs
- Wrote tests/chtrie.rs: add/del/query helpers (term[]/nchild[] arrays, N=65536, M=256), main test mirroring C test.c (dict1/dict2/stop/dict3, 14 queries), plus new(0,0) and walk NotFound tests.
- Fixed src/lib.rs bugs found by tests: free-pool logic inverted (idxpool now (1..n), idxptr = n-1, walk pops with idxptr-1, Capacity when idxptr==0 && idxmax>=maxn); ecap clamped to >=1 to avoid div-by-zero.
- cargo test: 3 passed; 0 failed.
