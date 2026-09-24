# CH-Trie: C → Rust Translation Design

## 1. Source project overview

CH-Trie is a small, self-contained C library implementing a **coordinate hash
trie** (a trie where each node's children are stored in a hash table of
linked lists, and node indices are recycled through a free pool).

Files (source root: `assets/ReCodeAgent/data/tool_projects/crust/chtrie/c`):

| File            | Role                                             |
|-----------------|--------------------------------------------------|
| `src/chtrie.h`  | Public API: `struct chtrie`, 4 functions         |
| `src/chtrie.c`  | Implementation (alloc / walk / del / free)       |
| `tests/test.c`  | `main()`-based integration test                  |
| `Makefile`      | Builds `runtest` and runs it                     |

There are **no third-party dependencies** — only the C standard library
(`<stddef.h>`, `<stdlib.h>`, `<limits.h>`, `<errno.h>`).

### Public API (C)

```c
chtrie *chtrie_alloc(size_t n, size_t m);
int     chtrie_walk(chtrie *tr, int from, int sym, int creat);
void    chtrie_del(chtrie *tr, int from, int sym);
void    chtrie_free(chtrie *tr);
```

- `chtrie_alloc(n, m)`: allocate a trie with at most `n` nodes and alphabet
  size `m`. Values `< 1` are clamped to 1. Returns `NULL` + sets `errno`
  (`ERANGE` for overflow, `ENOMEM` for allocation failure).
- `chtrie_walk(tr, from, sym, creat)`: follow/create the edge `(from, sym)`.
  Returns the child index, or `-1` on failure (sets `errno` when `creat` is
  set and the pool is exhausted).
- `chtrie_del(tr, from, sym)`: remove the edge `(from, sym)` and recycle the
  child index. No-op if the edge does not exist.
- `chtrie_free(tr)`: free all memory.

### Data structures (C)

```c
struct chtrie_edge { struct chtrie_edge *next; int from, sym, to; };

typedef struct {
    struct chtrie_edge **etab;   // hash table: array of linked-list heads
    int *idxpool, *idxptr, idxmax; // free-index pool (stack) + next fresh index
    int maxn, alphsz, ecap;
} chtrie;
```

Key invariants:
- `etab` has `ecap = (n-1) + (n-1)/3` slots (≈ 4/3·n), each a head of a
  singly-linked list of edges.
- `idxpool` is a stack of recycled node indices; `idxptr` is the stack top
  (points one past the last free slot). `idxmax` is the next fresh index.
- Hash: `h = (from * alphsz + sym) % ecap`.
- Node 0 is the root; indices are `0 .. maxn-1`.

## 2. Third-party dependency analysis

The C project depends **only on the C standard library**. There are no
external crates to map. The Rust translation uses only `std`:

| C dependency | Rust counterpart | Notes |
|--------------|------------------|-------|
| `<stddef.h>` (`size_t`) | `usize` | built-in |
| `<stdlib.h>` (`malloc`/`free`) | `Box` / `Vec` / allocator | built-in |
| `<limits.h>` (`INT_MAX`) | `i32::MAX` | built-in |
| `<errno.h>` (`errno`, `ERANGE`, `ENOMEM`) | `std::io::Error` / custom error enum | built-in |

**No `Cargo.toml` dependencies are required** (`[dependencies]` is empty).

## 3. Rust target design

### Crate layout

```
chtrie/
├── Cargo.toml
├── src/
│   ├── lib.rs        // public re-exports + ChTrieError
│   └── chtrie.rs     // Edge + ChTrie implementation
└── tests/
    └── test.rs       // integration test (translated from tests/test.c)
```

`Cargo.toml`:

```toml
[package]
name = "chtrie"
version = "0.1.0"
edition = "2021"

[dependencies]
```

### Types

```rust
/// A single directed edge in the trie.
pub struct Edge {
    next: Option<Box<Edge>>, // linked-list successor
    from: i32,
    sym:  i32,
    to:   i32,
}

/// A coordinate hash trie.
pub struct ChTrie {
    etab:   Vec<Option<Box<Edge>>>, // hash table of edge-list heads
    idxpool: Vec<i32>,              // recycled node indices (stack)
    idxptr:  usize,                 // number of free indices currently in pool
    idxmax:  i32,                   // next fresh index
    maxn:    i32,
    alphsz:  i32,
    ecap:    i32,
}
```

Design notes:
- `etab` is `Vec<Option<Box<Edge>>>` — each slot is the head of a linked
  list, exactly mirroring `struct chtrie_edge **etab`.
- `idxpool` + `idxptr` replace the C pointer-based stack: `idxptr` is the
  count of free indices. Push = `idxpool[idxptr] = to; idxptr += 1`;
  Pop = `idxptr -= 1; idxpool[idxptr]`.
- `Box<Edge>` replaces the C `malloc`'d edge nodes; `Vec` replaces the
  `malloc`'d arrays. Ownership is automatic — no manual `free`.

### Error type

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChTrieError {
    Range,   // n or m too large (C: ERANGE)
    Alloc,   // allocation failure / pool exhausted (C: ENOMEM)
}
impl std::fmt::Display for ChTrieError { ... }
impl std::error::Error for ChTrieError {}
```

### Public API (Rust)

```rust
impl ChTrie {
    /// Allocate a trie with at most `n` nodes and alphabet size `m`.
    /// Values < 1 are clamped to 1.
    pub fn alloc(n: usize, m: usize) -> Result<ChTrie, ChTrieError>;

    /// Follow (and optionally create) the edge `(from, sym)`.
    /// Returns `Some(child)` on success, `None` if the edge is absent and
    /// `creat` is false, or if creation is impossible (pool exhausted).
    pub fn walk(&mut self, from: i32, sym: i32, creat: bool) -> Option<i32>;

    /// Delete the edge `(from, sym)`, recycling the child index.
    /// No-op if the edge does not exist.
    pub fn del(&mut self, from: i32, sym: i32);
}

impl Drop for ChTrie { /* nothing needed: Box/Vec free themselves */ }
```

API mapping table:

| C function | Rust method | Return convention |
|------------|-------------|-------------------|
| `chtrie_alloc(n, m)` → `chtrie*` / `NULL`+errno | `ChTrie::alloc(n, m)` → `Result<ChTrie, ChTrieError>` | `Ok`/`Err` |
| `chtrie_walk(tr, from, sym, creat)` → `int` / `-1` | `tr.walk(from, sym, creat)` → `Option<i32>` | `Some`/`None` |
| `chtrie_del(tr, from, sym)` → `void` | `tr.del(from, sym)` | — |
| `chtrie_free(tr)` → `void` | `Drop` (automatic) | — |

`creat` is a `bool` in Rust (C used `int` 0/1).

### Implementation details to preserve

1. **`alloc` overflow checks** (must match C semantics):
   - Clamp `n` and `m` to at least 1.
   - If `n > i32::MAX as usize` or `m > i32::MAX as usize` → `Err(Range)`.
   - Compute `ecap = (n-1) + (n-1)/3`; if `ecap > i32::MAX as usize` →
     `Err(Range)`. (C: `MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1)/3`.)
   - Allocate `etab = vec![None; ecap]`, `idxpool = vec![0; n]`,
     `idxptr = 0`, `idxmax = 1`.
   - On `Vec` allocation failure (OOM) → `Err(Alloc)`. (In practice `Vec`
     aborts on OOM; we can note this but it is acceptable.)

2. **`walk`**:
   - Compute `h = ((from as u64) * (alphsz as u64) + (sym as u64)) % (ecap as u64)`.
   - Scan the linked list at `etab[h]` for an edge with `from == from && sym == sym`.
   - If found → `Some(edge.to)`.
   - If not found and `!creat` → `None`.
   - If not found and `creat`:
     - If `idxptr > 0`: pop a recycled index → `to`.
     - Else if `idxmax < maxn`: `to = idxmax; idxmax += 1`.
     - Else → `None` (pool exhausted; C sets `errno = ENOMEM`).
     - Create a new `Edge { next: None, from, sym, to }`, push it onto the
       head of `etab[h]`.
   - Return `Some(to)`.

3. **`del`**:
   - Compute `h` as above.
   - Walk the linked list at `etab[h]` to find the edge with `from == from && sym == sym`.
   - If found: unlink it (fix the `next` pointer of the predecessor, or set
     `etab[h] = None` if it was the head), recycle `edge.to` by pushing it
     onto the free pool (`idxpool[idxptr] = to; idxptr += 1`), and drop the
     `Box<Edge>`.
   - If not found: no-op.

4. **`Drop`**: nothing to do — `Vec` and `Box` free their memory automatically.

### Test translation (`tests/test.rs`)

The C test is a `main()` program. In Rust it becomes an integration test
using `#[test]`. The global `tr`, `term[N]`, `nchild[N]` become local
variables (or a small struct) inside the test function.

```rust
use chtrie::ChTrie;

const N: usize = 65536;
const M: usize = 256;

#[test]
fn test_chtrie() {
    let mut tr = ChTrie::alloc(N, M).unwrap();
    let mut term: Vec<i32> = vec![0; N];
    let mut nchild: Vec<i32> = vec![0; N];

    // add / del / query helpers as closures or local functions
    // ... (translate add, del, query from tests/test.c)

    // Add words
    for s in ["", "the", "a", "an"] { add(&mut tr, &mut term, &mut nchild, s); }
    for s in ["he", "she", "his", "hers"] { add(&mut tr, &mut term, &mut nchild, s); }
    for s in ["the", "an", "a"] { del(&mut tr, &mut term, &mut nchild, s); }
    for s in ["this", "that"] { add(&mut tr, &mut term, &mut nchild, s); }

    // Test cases
    let test_cases = ["hello", "the", "his", "he", "his", "go",
                      "he", "a", "an", "this", "that", "hey", "she", "hers"];
    let expected = [0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1];

    for (i, tc) in test_cases.iter().enumerate() {
        let result = query(&mut tr, tc);
        assert_eq!(result, expected[i], "Query: {}", tc);
    }
}
```

The `add`, `del`, and `query` helpers from the C test are translated
directly, using `tr.walk(from, sym, creat)` instead of `chtrie_walk(tr, ...)`.

## 4. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| **Linked-list mutation in `del`**: C uses pointer arithmetic to unlink; Rust uses `Option<Box<Edge>>` and requires careful `Option` chaining. | Use a `while let Some(edge) = ...` loop with a `prev` tracker, or rebuild the list. Keep it simple: iterate with indices into a temporary `Vec` of `Box<Edge>`, then reassemble. |
| **`Vec` OOM behavior**: C returns `NULL` on `malloc` failure; Rust `Vec` aborts on OOM. | Acceptable for this project. Document the difference. The `Alloc` error variant is still defined for API symmetry. |
| **`i32` vs `usize` overflow**: C uses `int` for indices; Rust uses `i32` to match. Hash computation uses `u64` to avoid overflow. | Use `u64` for the hash computation, cast back to `usize` for indexing. |
| **`creat` parameter type**: C uses `int` (0/1); Rust uses `bool`. | Straightforward mapping. |
| **Test globals**: C test uses file-scope globals; Rust test uses local variables. | Pass `&mut` references to helpers. |

## 5. Build & test

```bash
cargo build   # compile the library
cargo test    # run the integration test
```

No external crates, no `build.rs`, no feature flags.
