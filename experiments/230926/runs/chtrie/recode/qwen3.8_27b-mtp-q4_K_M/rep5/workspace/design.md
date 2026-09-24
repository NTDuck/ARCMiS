# CH-Trie: C → Rust Translation Design

## 1. Source Project Analysis

### Overview
CH-Trie is a C library implementing the **coordinate hash trie** — a trie variant that stores all edges in a single global hash table. The hash function is `h(from, sym) = (from * m + sym) mod H`, where `m` is the alphabet size and `H` is the table capacity. No rehashing or resizing is needed.

### File Structure
```
chtrie/
├── src/
│   ├── chtrie.h      # Public API (4 functions, 2 structs)
│   └── chtrie.c      # Implementation (~110 lines)
├── tests/
│   └── test.c        # Integration test (string set: add/del/query)
├── Makefile          # Build: compile .o, link test, run
├── README.md
└── LICENSE           # BSD 3-Clause
```

### Public API (C)
| Function | Signature | Returns |
|----------|-----------|---------|
| `chtrie_alloc` | `(size_t n, size_t m)` | `chtrie*` (NULL on error, sets errno) |
| `chtrie_walk` | `(chtrie*, int from, int sym, int creat)` | `int` (child index, or -1) |
| `chtrie_del` | `(chtrie*, int from, int sym)` | `void` |
| `chtrie_free` | `(chtrie*)` | `void` |

### Data Structures
```c
struct chtrie_edge {
    struct chtrie_edge *next;  // linked list for hash collisions
    int from, sym, to;         // edge: from node --sym--> to node
};

struct chtrie {
    struct chtrie_edge **etab;   // hash table: array of ecap pointers
    int *idxpool, *idxptr;       // pool of available node indices
    int idxmax;                  // next fresh index
    int maxn, alphsz, ecap;      // capacity, alphabet size, table size
};
```

### Key Invariants
- `ecap = (n-1) + (n-1)/3` (load factor ≈ 3/4)
- Root node is index 0; indices are in `[0, n)`
- Symbols are in `[0, m)`
- `idxptr` points into `idxpool`; freed indices are pushed back
- Edge table slots hold linked lists (head-insertion)

### Dependencies
**None.** The project uses only the C standard library (`stddef.h`, `stdlib.h`, `limits.h`, `errno.h`).

### Build & Test
- `make` → compiles `chtrie.o`
- `make test` → links `chtrie.o` + `tests/test.c`, runs the binary
- Test: builds a string set with add/del/query, asserts 14 expected results

## 2. Third-Party Library Mapping

The C project has **zero third-party dependencies**. All functionality comes from the C standard library.

| C Standard Library | Rust Equivalent | Notes |
|--------------------|-----------------|-------|
| `stddef.h` (size_t, NULL) | `std::usize`, `Option<T>` | Built-in |
| `stdlib.h` (malloc, calloc, free) | `Vec<T>`, `Box<T>`, `Drop` | RAII replaces manual free |
| `limits.h` (INT_MAX) | `usize::MAX`, `isize::MAX` | Built-in |
| `errno.h` (errno, ERANGE, ENOMEM) | `std::io::Error` or custom `Error` enum | Idiomatic Rust error handling |

**No external crates are needed.** The translation uses only `std`.

## 3. Target Project Design (Rust)

### Module Structure
```
chtrie/
├── Cargo.toml
├── src/
│   └── lib.rs          # All code: ChTrie struct, Edge struct, error type, tests
├── tests/
│   └── integration.rs  # Integration test mirroring tests/test.c
├── README.md
└── LICENSE
```

Since the library is small (~110 lines of C), a single `lib.rs` is appropriate. No need to split into multiple modules.

### Type Mapping

| C | Rust | Rationale |
|---|------|-----------|
| `struct chtrie_edge` | `struct Edge` | Private, internal to the crate |
| `struct chtrie` | `pub struct ChTrie` | Public API type |
| `struct chtrie_edge **etab` | `Vec<Option<Edge>>` | Array of optional edge heads (None = empty slot) |
| `int *idxpool, *idxptr` | `Vec<usize>` + `usize` (ptr index) | Pool of recycled indices |
| `int idxmax` | `usize` | Next fresh index |
| `int maxn, alphsz, ecap` | `usize` | Capacities |
| `int from, sym, to` | `usize` | Node/symbol indices (non-negative) |
| `int creat` (0/1) | `bool` | Idiomatic Rust |
| `int` return, -1 = error | `Option<usize>` | None = not found / failed |
| `errno` + NULL return | `Result<ChTrie, ChTrieError>` | Idiomatic Rust |

### Public API (Rust)

```rust
/// Error type for ChTrie operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChTrieError {
    /// The requested size exceeds the maximum representable value.
    Range,
    /// The trie has no more available node indices.
    Capacity,
}

impl std::fmt::Display for ChTrieError { ... }
impl std::error::Error for ChTrieError { ... }

/// A coordinate hash trie.
pub struct ChTrie {
    etab: Vec<Option<Edge>>,
    idxpool: Vec<usize>,
    idxptr: usize,       // next position in idxpool to pop from
    idxmax: usize,       // next fresh index to allocate
    maxn: usize,
    alphsz: usize,
    ecap: usize,
}

impl ChTrie {
    /// Allocate a trie with at most `n` nodes and alphabet size `m`.
    /// If `n` or `m` is 0, they are regulated to 1.
    pub fn new(n: usize, m: usize) -> Result<Self, ChTrieError>;

    /// Walk from node `from` via symbol `sym`.
    /// If `create` is true and the edge doesn't exist, create it.
    /// Returns the child node index, or None if not found and create is false.
    pub fn walk(&mut self, from: usize, sym: usize, create: bool) -> Option<usize>;

    /// Delete the edge from `from` via `sym`.
    /// The child must be a leaf (caller's responsibility).
    /// If the edge doesn't exist, the trie is unchanged.
    pub fn delete(&mut self, from: usize, sym: usize);
}

impl Drop for ChTrie {
    fn drop(&mut self) { /* Vec handles cleanup automatically */ }
}
```

### Internal Edge Struct
```rust
#[derive(Clone)]
struct Edge {
    from: usize,
    sym: usize,
    to: usize,
}
```

Note: In C, edges form a linked list via `next` pointer. In Rust, we use `Vec<Option<Edge>>` where each slot is a **single edge** (head of the chain). For collision chains longer than 1, we need a different approach:

**Option A**: Use `Vec<Vec<Edge>>` — each slot is a small vector of edges (typically 0-1 elements).
**Option B**: Use `Vec<Option<Edge>>` with a separate `next: Option<usize>` field pointing to the next slot index.

I'll go with **Option A** (`Vec<Vec<Edge>>`) because:
- It's simpler and more idiomatic Rust
- Collision chains are very short (load factor 3/4)
- No need for manual linked-list management
- `Vec` handles growth/shrinkage automatically

Actually, reconsidering: the C code uses a fixed-size table with linked lists. The key property is **no reallocation**. In Rust, `Vec<Vec<Edge>>` would work fine — the outer `Vec` is fixed-size (allocated once in `new()`), and inner `Vec`s are tiny (0-2 elements typically). This preserves the "no rehashing" property.

### Algorithm Translation

#### `chtrie_alloc` → `ChTrie::new`
```
1. Clamp n, m to at least 1
2. Check n <= usize::MAX (trivially true) and overflow check for ecap
3. ecap = (n-1) + (n-1)/3
4. etab = vec![Vec::new(); ecap]
5. idxpool = vec![0; n]  (all zeros, but we track idxptr)
6. idxptr = 0, idxmax = 1
```

#### `chtrie_walk` → `ChTrie::walk`
```
1. h = (from.wrapping_mul(alphsz).wrapping_add(sym)) % ecap   // matches C `unsigned long` wrap-then-mod
2. Search etab[h] for edge where from==from && sym==sym
3. If found, return Some(to)
4. If create:
   a. Check capacity: if idxptr == idxpool.len() && idxmax >= maxn → return None
   b. Allocate to: if idxptr > 0 { to = idxpool[idxptr-1]; idxptr -= 1 } else { to = idxmax; idxmax += 1 }
   c. Insert Edge { from, sym, to } at etab[h].insert(0, edge)  (head insertion)
   d. Return Some(to)
5. Return None
```

#### `chtrie_del` → `ChTrie::delete`
```
1. h = (from * alphsz + sym) % ecap
2. Find and remove edge from etab[h]
3. If found: push `to` back onto idxpool (idxpool[idxptr] = to; idxptr += 1)
```

#### `chtrie_free` → `Drop`
Rust's `Drop` trait handles cleanup automatically. `Vec` deallocates its contents. No explicit free needed.

### Test Translation

The C test (`tests/test.c`) becomes `tests/integration.rs`:
- Build a string set using `ChTrie`
- Add words from dict1, dict2, dict3
- Delete words from stop list
- Query 14 test cases and assert expected results
- Use `assert_eq!` instead of C `assert`

Additionally, add unit tests in `lib.rs`:
- Test `new` with edge cases (n=0, m=0, very large values)
- Test `walk` with create and without
- Test `delete` for existing and non-existing edges
- Test index recycling (delete then re-add)

### Cargo.toml
```toml
[package]
name = "chtrie"
version = "0.1.0"
edition = "2021"
description = "Coordinate hash trie"
license = "BSD-3-Clause"

[dependencies]
# None — std only
```

## 4. Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Overflow in hash computation**: `from * alphsz + sym` could overflow `usize` for very large values | Medium | Use `wrapping_mul`/`wrapping_add` then `% ecap`, exactly mirroring C's `unsigned long` wrap-then-mod semantics (C relies on unsigned wraparound) |
| **Index pool semantics**: C uses a pointer into an array; Rust needs careful index management | Low | Use `Vec<usize>` + `usize` index; test thoroughly with add/delete cycles |
| **Head insertion order**: C inserts at head of linked list; Rust `Vec::insert(0, ...)` is O(n) | Low | Collision chains are very short (load factor 3/4), so O(n) ≈ O(1) in practice |
| **`int` vs `usize`**: C uses signed `int` for indices; Rust uses unsigned `usize` | Low | All indices are non-negative by contract; use `usize` throughout |
| **Error handling**: C uses `errno` + NULL/-1; Rust uses `Result`/`Option` | Low | Define `ChTrieError` enum; use `Result` for `new`, `Option` for `walk` |
| **Memory safety**: C has potential for use-after-free, double-free | High (in C) | Rust's ownership system eliminates these entirely |
| **`Drop` ordering**: C requires explicit `chtrie_free`; Rust `Drop` is automatic | Low | Document that explicit free is not needed; `Drop` handles it |

### Key Design Decisions
1. **`Vec<Vec<Edge>>` over linked lists**: More idiomatic Rust, avoids manual pointer management. The "no reallocation" property is preserved because the outer `Vec` is fixed-size.
2. **`Option<usize>` for `walk`**: Cleaner than `Result<usize, ChTrieError>` for the common "not found" case. Capacity errors are rare and could be a separate check.
3. **`bool` for `create`**: More readable than C's `int creat`.
4. **Single `lib.rs`**: The library is small enough that splitting into modules adds unnecessary complexity.
5. **No external dependencies**: The C project has none, and Rust's `std` covers all needs.
