# Brainstorm: CH-Trie C → Rust Translation

Scope: translate `src/chtrie.h` / `src/chtrie.c` (a CH-Trie, i.e. a compact
hash trie over coordinate pairs `(from, sym)`) plus `tests/test.c` into a
Rust crate that builds and passes `cargo test`.

---

## 1. Core difficulties of translating this specific C code

### 1.1 Raw pointers and manual memory management

The C code allocates:

- `etab`: an array of `edge *` bucket heads (calloc'd, zeroed).
- Each `edge`: malloc'd on demand, singly linked via `edge->next`.
- `idxpool`: a `uint32_t *` array of recycled node indices.
- `chtrie_free` recursively walks the bucket lists and frees every edge.

Rust modeling:

- `etab: Vec<Option<Box<Edge>>>` — one bucket per slot; `None` = empty
  bucket (the C `NULL` head). `Box<Edge>` owns each node; `Edge.next:
  Option<Box<Edge>>` replaces the `edge *next` self-link. This is the
  natural, leak-free, double-free-free translation: dropping the `ChTrie`
  recursively drops the whole edge forest. `chtrie_free` becomes `impl Drop`
  (or simply the implicit drop — no manual walk needed).
- Alternative: `Vec<Vec<Edge>>` (one `Vec` per bucket) — simpler, but
  changes allocation behavior and makes "first edge in bucket" ordering
  depend on `Vec` push order rather than C prepend order. Prefer the
  `Box`-linked list for fidelity.
- `idxpool: Vec<usize>` (or `Vec<u32>`) + `next_idx: usize` replaces
  `idxpool` array + `idxptr` moving pointer + `idxmax`. The C code grows
  the pool with `realloc` when `idxptr == idxmax`; in Rust the `Vec` grows
  itself, so the `realloc`/`ENOMEM` path disappears (or is modeled as
  `Err(Alloc)` if we want to mirror it — see 1.3).

### 1.2 The `chtrie` struct layout

C fields and their Rust counterparts:

| C field | Meaning | Rust |
|---|---|---|
| `edge **etab` | `ecap` bucket heads | `Vec<Option<Box<Edge>>>` |
| `uint32_t *idxpool` | recycled node indices | `Vec<usize>` |
| `uint32_t *idxptr` | moving pointer into pool | implicit: `Vec::pop()` / `Vec::push()` (LIFO) |
| `uint32_t idxmax` | pool capacity | implicit: `Vec` capacity |
| `uint32_t maxn` | max node count | `usize` |
| `uint32_t alphsz` | alphabet size `m` | `usize` |
| `uint32_t ecap` | edge-table size `(n-1)+(n-1)/3` | `usize` |

Key subtleties:

- **Root is index 0** and is never pushed into `idxpool`. The C code
  allocates node 0 at init and starts `idxptr` at 1. Rust: `next_idx`
  starts at 1; `del` must never recycle index 0.
- **`ecap` formula**: `ecap = (n-1) + (n-1)/3` (integer division). Must be
  reproduced exactly — it determines the hash bucket of every edge and
  therefore the observable edge order in bucket lists.
- **Hash**: `h = (from * alphsz + sym) % ecap`. In C this is 32-bit
  arithmetic; in Rust we should compute in `usize` (or `u64`) with checked
  arithmetic to avoid silent wraparound differences. Since `n, m <= i32::MAX`
  is enforced, `from * alphsz` fits in `u64` comfortably.
- **`walk` with `creat=1`** allocates a new node index when the edge is
  missing: C does `*idx = *idxptr++; if (*idxptr > idxmax) realloc...` —
  i.e. fresh indices are handed out in increasing order, and recycled
  indices are popped LIFO from the pool. This exact allocation order is
  observable (indices are returned to the caller), so it must be preserved.

### 1.3 errno semantics → Rust `Result`

C signals failure via `errno`:

- `ERANGE` from `chtrie_alloc` when `n` or `m` is out of range (the C code
  checks against `UINT32_MAX`-ish limits and the `ecap` overflow).
- `ENOMEM` from `chtrie_alloc` (calloc failure) and from `chtrie_walk`
  when the node index space is exhausted (`idxptr` reaches `maxn` with an
  empty pool) — note the C code sets `errno = ENOMEM` in the exhaustion
  path, not only on real malloc failure.
- `chtrie_walk` with `creat=0` on a missing edge returns `-1` (no errno).
- `chtrie_del` on a missing edge is a silent no-op.

Rust mapping — a typed error enum is cleaner than mirroring errno:

```rust
pub enum ChTrieError {
    Range,      // <- ERANGE (n/m too large, ecap overflow)
    Capacity,   // <- ENOMEM from node-index exhaustion in walk(creat)
    Alloc,      // <- ENOMEM from real allocation failure (rare in Rust)
    NotFound,   // <- walk(creat=0) miss
}
```

How tests observe errors: C tests can only check return values (the
provided `test.c` checks results, not errno). Rust tests can match on the
`Err` variant directly — strictly better. We should still keep the variant
semantics aligned with the C errno choices so behavior is equivalent.

One C quirk to decide on: the C `walk` malloc-failure path reportedly does
**not** set `errno` (bug). In Rust we return `Err(Alloc)` consistently —
a deliberate, documented deviation that only matters under OOM.

### 1.4 C89 constraints vs Rust idioms

- C89: declarations at block top, `uint32_t` arithmetic, no generics.
  Rust: none of these constrain us; the only real constraint is **not
  changing observable behavior** (index allocation order, bucket order,
  error conditions).
- C `calloc` zero-init → Rust `vec![None; ecap]` / `Edge { next: None, .. }`.
- C `realloc` growth of `idxpool` → Rust `Vec` amortized growth; the
  `ENOMEM`-on-realloc path effectively vanishes (we can keep an `Alloc`
  variant for API symmetry).
- C `free` + manual recursion in `chtrie_free` → `Drop` (implicit).
- C `assert`-free library (errors via errno) → Rust `Result`; internal
  invariants can use `debug_assert!` (e.g. `next_idx <= maxn`, root never
  pooled).

### 1.5 Porting `tests/test.c`

The C test builds a `StringSet`-style helper (a `chtrie` plus per-node
`term[]` and `nchild[]` arrays), adds a fixed set of words, then asserts a
fixed expected-results array (14 queries: hello=0, the=0, his=1, he=1,
his=1, go=0, he=1, a=0, an=0, this=1, that=1, hey=0, she=1, hers=1 —
1 = "is a prefix of some stored word").

Porting strategy:

- Reimplement the helper as a small Rust struct `StringSet` wrapping
  `ChTrie` + `term: Vec<bool>` + `nchild: Vec<usize>`, with `add`, `del`,
  `is_prefix` methods mirroring the C test helper.
- One `#[test] fn test_c_suite()` running the exact 14 assertions in the
  exact order (order matters: it exercises `del` + index reuse mid-suite).
- Additional `#[test]` functions for error paths the C test doesn't cover:
  `new(0,0)` clamping, `new` overflow → `Range`, `walk` miss → `NotFound`,
  `del` of nonexistent edge (no-op), LIFO index reuse after `del`.
- `assert!` maps 1:1 to `assert!`/`assert_eq!`; the fixed expected array
  becomes a `&[(bool, bool)]` table iterated in a loop, or explicit
  `assert_eq!` lines for readability.

### 1.6 `del` index-pool reuse semantics (must be exact)

C `chtrie_del(from, sym)`:

1. Find the edge in its bucket list; if absent → return (no-op).
2. Unlink the edge, `free` it.
3. If the target node `to` has no other incoming edges (i.e. its child
   count drops to 0) and is not the root, push `to` onto `idxpool`
   (`*idxptr++ = to`), and **recursively** prune `to`'s own outgoing edges
   (each of their targets is also freed/pooled if orphaned).
4. Pooled indices are later handed out **LIFO** by `walk(creat=1)`:
   the most recently freed index is the next one allocated.

Behavioral equivalence requirements:

- The **set** of live indices and the **LIFO order** of the pool must match
  C exactly, because `walk` returns the allocated index to the caller and
  the test helper stores `term`/`nchild` arrays indexed by node index.
- Root (0) is never pooled.
- A node is pooled only when it becomes a leaf *and* is non-terminal in
  the C helper's bookkeeping — actually, in the C library `del` pools the
  target unconditionally when its edge is removed (the library doesn't know
  about `term`); the *helper* in test.c tracks `term`/`nchild` separately.
  We must mirror the library's rule: pool `to` when its last incoming edge
  is removed, then recursively prune `to`'s outgoing edges.
- Rust implementation: `idxpool: Vec<usize>` with `push` (free) and `pop`
  (reuse); `next_idx: usize` for fresh allocations. `walk(creat)`:
  `let idx = self.idxpool.pop().unwrap_or_else(|| { let i = self.next_idx;
  self.next_idx += 1; i });` with a `Capacity` error when `next_idx > maxn`
  and the pool is empty.

---

## 2. Candidate approaches

### A) Direct port (recommended)

Keep the exact data structures and algorithms, line for line:

- `etab: Vec<Option<Box<Edge>>>` with `Edge { next: Option<Box<Edge>>,
  from, sym, to }` — same separate-chaining hash table, same prepend
  order, same bucket computation.
- `idxpool: Vec<usize>` + `next_idx: usize` — same LIFO reuse.
- Same `ecap` formula, same hash, same clamp/overflow checks.
- `new` / `walk` / `del` / `Drop` map 1:1 to `chtrie_alloc` /
  `chtrie_walk` / `chtrie_del` / `chtrie_free`.

**Pros:** maximal behavioral fidelity (index allocation order, bucket
order, error conditions all match C); easiest to verify against the C
test's fixed expectations; smallest cognitive diff; `del`'s recursive
pruning and LIFO pool carry over directly.

**Cons:** keeps C-shaped structures (linked lists of `Box`) that are less
idiomatic; slightly more code than a `HashMap` version; `Box` chains are
cache-unfriendly (irrelevant at this scale).

### B) Idiomatic rewrite

- `edges: HashMap<(usize, usize), usize>` (from,sym → to) plus
  `children: HashMap<usize, usize>` for in-degree counts.
- `Vec<usize>` free list for index reuse.

**Pros:** much shorter code; no manual list unlinking; `del` becomes
`edges.remove` + in-degree decrement.

**Cons:** loses bucket-list ordering (not observable through the public
API, but changes internal behavior); `HashMap` iteration order is
nondeterministic — if any code path enumerates edges, behavior diverges;
the in-degree bookkeeping is a new invariant the C code doesn't have,
adding a new bug surface; harder to argue "same behavior as C".

### C) Hybrid

Keep the open-addressing/linked-list hash table for fidelity but use
Rust safe types (`Vec<Option<Box<Edge>>>`), and use idiomatic Rust only
where C has no observable behavior (e.g. `Vec` growth instead of
`realloc`, `Drop` instead of manual free).

This is effectively **A** with the explicit principle "deviate only where
C has no observable behavior". It is the same recommendation as A, stated
as a policy.

### Recommendation

**Approach A/C (direct port with safe types).** The test suite pins
observable behavior (returned indices, prefix results, error conditions),
and the C code is small enough that a faithful port is both the safest and
the shortest path to a passing, verifiable translation.

---

## 3. Concrete crate layout

```
Cargo.toml          # name = "chtrie", edition = "2021", no deps
src/lib.rs          # pub mod chtrie; pub use chtrie::{ChTrie, ChTrieError};
src/chtrie.rs       # ChTrieError, Edge, ChTrie, impl ChTrie, #[cfg(test)] tests
tests/test.rs       # (optional) integration test porting tests/test.c
```

Public API:

```rust
pub enum ChTrieError { Range, Capacity, Alloc, NotFound }

pub struct ChTrie { /* etab, idxpool, next_idx, maxn, alphsz, ecap */ }

impl ChTrie {
    /// chtrie_alloc: clamps n,m >= 1; Err(Range) on overflow.
    pub fn new(n: usize, m: usize) -> Result<Self, ChTrieError>;

    /// chtrie_walk: find edge (from, sym); if missing and creat,
    /// allocate a node index (LIFO pool first, then fresh) and insert.
    /// Returns the target index; Err(NotFound) if missing & !creat;
    /// Err(Capacity) if index space exhausted.
    pub fn walk(&mut self, from: usize, sym: usize, creat: bool)
        -> Result<usize, ChTrieError>;

    /// chtrie_del: remove edge (from, sym); no-op if absent.
    /// Recursively prunes orphaned subtrees, pooling indices LIFO.
    pub fn del(&mut self, from: usize, sym: usize);
}
// chtrie_free -> implicit Drop (recursive Box drop)
```

Naming note: `new`/`walk`/`del` + `Drop` is the idiomatic Rust shape of
`alloc`/`walk`/`del`/`free`; keep `walk` and `del` verbatim since they
are the algorithm's vocabulary.

Test plan (in `#[cfg(test)]` in `src/chtrie.rs` and/or `tests/test.rs`):

1. `test_c_suite` — port of `tests/test.c`: build `StringSet`, add the
   C test's words, run the 14 prefix assertions in order (including the
   `del` steps the C test performs), assert the exact expected array.
2. `test_new_clamps_zero` — `new(0,0)` succeeds, behaves like `new(1,1)`.
3. `test_new_range` — `new(i32::MAX as usize + 1, 1)` → `Err(Range)`;
   `new` with `ecap` overflow → `Err(Range)`.
4. `test_walk_miss` — `walk(x, y, false)` on missing edge → `Err(NotFound)`.
5. `test_del_missing_noop` — `del` of absent edge changes nothing.
6. `test_lifo_reuse` — allocate two nodes, `del` both (in known order),
   then `walk(creat)` must return the indices in LIFO free order.
7. `test_capacity` — small `n`, exhaust the index space → `Err(Capacity)`.

---

## 4. Risks and edge cases

| # | Edge case | Expected behavior | Risk |
|---|---|---|---|
| 1 | `n = 0` or `m = 0` | clamped to 1 (C does `if (n < 1) n = 1`) | forgetting the clamp → `ecap = 0` → modulo-by-zero panic in Rust (C would also misbehave) |
| 2 | `n` huge (e.g. `usize::MAX`) | `Err(Range)` before any allocation | computing `ecap` before the range check → overflow panic; must check `n > i32::MAX` **first** |
| 3 | `ecap` overflow: `(n-1) + (n-1)/3 > i32::MAX` | `Err(Range)` | same as above; use checked arithmetic |
| 4 | Node exhaustion: `walk(creat)` with `next_idx > maxn` and empty pool | `Err(Capacity)` (C: `errno = ENOMEM`) | off-by-one on `maxn` (inclusive vs exclusive) — C's `idxptr` starts at 1 and the bound is `maxn`; verify with a small-`n` test |
| 5 | `del` of nonexistent edge | silent no-op | accidentally returning an error or panicking |
| 6 | Index reuse after `del` | LIFO: last freed index is next allocated | using a FIFO queue or a `HashSet`-based allocator breaks the C test's index-dependent `term`/`nchild` arrays |
| 7 | Root index 0 | never pooled, never freed | `del` of an edge into root (shouldn't happen) or recursive pruning reaching 0 must stop |
| 8 | Recursive pruning in `del` | a node is pooled only when its last incoming edge is removed; its outgoing edges are then removed and their targets pruned | double-pooling a node (pushed twice) → later `walk` returns a dead index; guard with in-degree check |
| 9 | Bucket-list order | C prepends new edges; lookup scans head-first | using `Vec::push` per bucket reverses order — unobservable through the API, but keep prepend for fidelity |
| 10 | Hash arithmetic | `h = (from * alphsz + sym) % ecap` in 32-bit C | Rust `usize` math is fine given the `i32::MAX` bound, but use `u64`/checked math to be safe |
| 11 | Empty-string add in the test helper | sets `term[0] = 1`, no edges | helper port must special-case `len == 0` |
| 12 | `walk` malloc-failure path | C bug: no `errno` set; Rust returns `Err(Alloc)` | documented deviation; only observable under OOM |

---

## Decision

Proceed with **Approach A (direct port, safe Rust types)**:
`Vec<Option<Box<Edge>>>` edge table, `Vec<usize>` LIFO index pool +
`next_idx` counter, `ChTrieError { Range, Capacity, Alloc, NotFound }`,
`new`/`walk`/`del` + `Drop`, tests ported 1:1 plus the extra edge-case
tests above.
