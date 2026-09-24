# CH-Trie → Rust Translation Design

## 1. Source Project Analysis

**Project:** CH-Trie — the official C library of the *coordinate hash trie* (BSD 3-Clause, DONG Yuxuan).

**Structure (C89, POSIX `errno`):**

| File | Role |
|---|---|
| `src/chtrie.h` | Public API + opaque-ish struct definitions |
| `src/chtrie.c` | Implementation (~110 lines) |
| `tests/test.c` | Test harness: builds a string set via add/del/query helpers, asserts 14 query results |
| `test.c_old` | Older interactive version (reads stdin) — not part of the test suite |
| `Makefile` | Builds `chtrie.o`, links `tests/test.c` into `run.tmp`, runs it; `make install` |
| `test.in` / `test.out` | I/O fixtures for the old interactive test (not used by `tests/test.c`) |

**Data structure:**
- A global hash table `etab` of `ecap = (n-1) + (n-1)/3` slots (load factor 3/4), **no rehashing/resize**.
- Each slot is the head of a singly linked list of edge nodes `(from, sym) -> to`, allocated individually with `malloc`.
- Hash: `h = (from * alphsz + sym) % ecap` (computed in `unsigned long`).
- Node indices: root = 0; new nodes allocated from `idxmax++`; freed indices recycled into an `idxpool` stack (`idxptr` points at the top).

**Public API (C):**
```c
chtrie *chtrie_alloc(size_t n, size_t m);          // NULL + errno (ERANGE) on failure
int chtrie_walk(chtrie *tr, int from, int sym, int creat); // -1 on miss/fail; errno ENOMEM on capacity
void chtrie_del(chtrie *tr, int from, int sym);    // no-op if edge absent
void chtrie_free(chtrie *tr);
```

**Semantics to preserve:**
1. `n < 1` / `m < 1` are clamped to 1.
2. `ERANGE` if `n > INT_MAX`, `m > INT_MAX`, or if `ecap` would overflow (`(n-1) + (n-1)/3` must fit in `isize`/pointer-sized allocation).
3. `walk` with `creat=0` returns the child index or -1 (no error side effect).
4. `walk` with `creat=1` creates a node; fails with `ENOMEM` when the pool is exhausted and `idxmax >= maxn`.
5. `del` removes the edge and recycles the child index into the pool; no-op if absent.
6. `free` releases all edges, `etab`, `idxpool`, and the struct.
7. The test suite (`tests/test.c`) is the acceptance criterion: 14 query assertions over a dict/stop-word string set.

**Dependencies:** none beyond the C standard library (`stddef.h`, `stdlib.h`, `limits.h`, `errno.h`).

## 2. Third-Party Library Mapping

The C project has **zero third-party dependencies** (libc only). The Rust translation therefore needs **no external crates** — everything maps to `std`:

| C dependency | Rust counterpart |
|---|---|
| `stdlib.h` (`malloc`/`calloc`/`free`) | `Vec` / `Box` (allocator managed by the runtime) |
| `errno.h` (`ERANGE`, `ENOMEM`) | custom `ChTrieError` enum implementing `std::error::Error` |
| `limits.h` (`INT_MAX`) | `u32::MAX` (node/symbol indices are `u32`) |
| `assert.h` (tests) | `assert!` / `#[test]` |

## 3. Target Project Design (Rust)

**Crate layout:**
```
chtrie/
├── Cargo.toml          # name = "chtrie", edition = "2021", no dependencies
├── src/
│   └── lib.rs          # ChTrie, Edge, ChTrieError, public API, unit tests
└── tests/
    └── test.rs         # port of tests/test.c (integration test)
```

**Types:**
```rust
/// One edge in the global hash table. `next` is an index into the
/// crate-internal edge pool (replaces the C linked-list pointers).
struct Edge {
    next: Option<usize>,   // next edge in the bucket chain
    from: u32,
    sym: u32,
    to: u32,
}

pub struct ChTrie {
    etab: Vec<Option<usize>>,   // bucket heads -> index into `edges`
    edges: Vec<Edge>,           // edge pool (replaces per-node malloc)
    idxpool: Vec<u32>,          // recycled node indices (stack)
    idxptr: usize,              // top of the pool
    idxmax: u32,                // next fresh index
    maxn: u32,
    alphsz: u32,
    ecap: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChTrieError {
    Range,      // n/m too large or ecap overflow (C: ERANGE)
    Capacity,   // node pool exhausted (C: ENOMEM)
}
impl std::fmt::Display for ChTrieError { ... }
impl std::error::Error for ChTrieError {}
```

**API (Rust idioms, `&mut self` for mutation, `Result` for failure):**
```rust
impl ChTrie {
    /// C: chtrie_alloc(n, m). Clamps n,m to >= 1.
    pub fn new(n: u32, m: u32) -> Result<Self, ChTrieError>;

    /// C: chtrie_walk(tr, from, sym, creat).
    /// creat=false: Ok(child) or Err(ChTrieError::NotFound) —
    ///   better: return Option<u32> for the non-creating form.
    /// Design choice: two methods for clarity:
    pub fn walk(&self, from: u32, sym: u32) -> Option<u32>;
    pub fn walk_or_create(&mut self, from: u32, sym: u32) -> Result<u32, ChTrieError>;

    /// C: chtrie_del. No-op if the edge is absent.
    pub fn del(&mut self, from: u32, sym: u32);
}
// C: chtrie_free -> Drop impl (Vecs free themselves; no manual Drop needed).
```

**Implementation notes / C→Rust mapping:**
- `calloc` → `vec![None; ecap]`, `vec![0u32; n]` (zero-init is inherent).
- Linked-list edge nodes → `edges: Vec<Edge>` with `next: Option<usize>` indices; `del` removes by swapping or by leaving the slot (simplest: keep the slot, just unlink from the chain — memory is reclaimed by `Vec` on drop; to mirror C's `free`, we can `mem::replace` with a dummy and `pop` if it is the last element, but correctness does not require it).
- Hash computation: `let h = ((from as u64) * (self.alphsz as u64) + sym as u64) % self.ecap as u64;` — use `u64` to avoid overflow (C relied on `unsigned long` wraparound; with `u32` bounds the product fits in `u64` safely).
- Overflow guard in `new`: check `n` and `m` against `u32::MAX` (C checked `INT_MAX`), and check `(n-1) + (n-1)/3` fits in `usize` before allocating (C's `MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1)/3` check).
- `errno` side effects → `Result` return values; no global state.
- `chtrie_free` → `Drop` (automatic via `Vec`).

**Tests (port of `tests/test.c`):**
- `tests/test.rs`: replicate the string-set harness (`add`, `del`, `query` over `u8` symbols, `N=65536`, `M=256`), same 14 test cases and expected results, using `assert_eq!`.
- Optional unit tests in `lib.rs`: alloc clamping, ERANGE on huge `n`, walk create/miss, del no-op, index recycling after del.

**Build/test:** `cargo test` (matches the required test command). No `Makefile` needed.

## 4. Risks & Mitigations

| Risk | Mitigation |
|---|---|
| **Overflow semantics differ**: C wraps `unsigned long`; Rust panics on overflow in debug builds. | Compute hash in `u64` (product of two `u32`s fits); use `checked_add`/`checked_mul` in `new` for the `ecap` guard. |
| **Error model**: C uses `errno` + `NULL`/`-1`; Rust uses `Result`/`Option`. | Define `ChTrieError::{Range, Capacity}`; `walk` (non-creating) returns `Option<u32>` (miss is not an error), `walk_or_create` returns `Result`. Document the mapping. |
| **Index type**: C uses `int` (32-bit) for node/symbol indices. | Use `u32` for `from`/`sym`/`to` and `maxn`/`alphsz`; `usize` for `Vec` indices. Bounds-check in debug; the C code assumed valid indices (UB otherwise) — Rust will panic on out-of-range, which is acceptable and safer. |
| **Edge pool memory**: C frees individual edges on `del`; Rust `Vec` keeps capacity. | Not a correctness issue; memory is reclaimed on `Drop`. Optionally compact the pool, but unnecessary. |
| **API shape change** (C functions → methods) may break drop-in C-style usage. | This is expected in a Rust port; provide a clear, idiomatic API and document the correspondence to the C functions. |
| **Test fidelity**: `tests/test.c` uses `assert` (aborts on failure). | Use `assert_eq!` in `#[test]` functions so `cargo test` reports failures properly. |
| **`test.c_old` / `test.in` / `test.out`** are legacy interactive fixtures. | Do not port them; they are not part of the `cargo test` acceptance set. Only `tests/test.c` is the reference. |

## 5. Acceptance

`cargo test` must pass, with the integration test reproducing the 14 query assertions from `tests/test.c` (expected: `0,0,1,1,1,0,1,0,0,1,1,0,1,1`).
