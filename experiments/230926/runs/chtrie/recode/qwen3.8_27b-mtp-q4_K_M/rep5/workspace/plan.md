# CH-Trie: C → Rust Implementation Plan

Translate the C coordinate-hash-trie library into idiomatic Rust. The design
(`design.md`) is authoritative. Skeletons already exist and compile; each step
below fills in one unit. Work strictly bottom-up: a unit is implemented only
after the units it depends on.

## Name Mapping (C → Rust)

| C symbol | Rust symbol | Why it changes |
|----------|-------------|----------------|
| `struct chtrie_edge` | `struct Edge` | Private internal type; drop the `chtrie_` prefix, idiomatic PascalCase |
| `struct chtrie` (typedef `chtrie`) | `pub struct ChTrie` | Public API type; PascalCase |
| `chtrie_alloc` | `ChTrie::new` | Constructor idiom; `errno`+`NULL` → `Result<Self, ChTrieError>` |
| `chtrie_walk` | `ChTrie::walk` | Method on the type; `int` return (`-1`=fail) → `Option<usize>` |
| `chtrie_del` | `ChTrie::delete` | Method; `del`→`delete` for clarity |
| `chtrie_free` | `impl Drop for ChTrie` | RAII replaces manual free |
| `errno` / `ERANGE` / `ENOMEM` | `ChTrieError::{Range, Capacity}` | Idiomatic Rust error enum |
| `int creat` (0/1) | `bool create` | Idiomatic boolean |
| `int from, sym, to` | `usize` | Non-negative indices |
| `struct chtrie_edge *next` (linked list) | `Vec<Vec<Edge>>` (per-slot chain) | No manual pointers; `Vec` owns the chain |
| `int *idxpool, *idxptr` | `Vec<usize>` + `usize idxptr` | Pool stack + top pointer |
| `tests/test.c` `main` | `#[test] fn string_set` | Rust test harness |
| `tests/test.c` `add`/`del`/`query` | `StringSet::{add, del, query}` | Helpers become methods on a test-local struct |

## Part A — Source files (bottom-up dependency order)

All in `src/lib.rs` (single-module design). Order: types → constructor →
operations → unit tests.

1. **`ChTrieError` + `Display` + `Error` impls** — no dependencies.
   - `Range`, `Capacity` variants; `Display` messages; `std::error::Error`.

2. **`struct Edge`** — no dependencies.
   - Fields `from, sym, to: usize`; `#[derive(Clone)]`.

3. **`struct ChTrie`** — depends on `Edge`.
   - Fields: `etab: Vec<Vec<Edge>>`, `idxpool: Vec<usize>`, `idxptr: usize`,
     `idxmax: usize`, `maxn: usize`, `alphsz: usize`, `ecap: usize`.

4. **`ChTrie::new(n, m) -> Result<Self, ChTrieError>`** — depends on 1–3.
   - Clamp `n`, `m` to ≥ 1.
   - Overflow-check `ecap = (n-1) + (n-1)/3`; return `Err(Range)` on overflow.
   - `etab = vec![Vec::new(); ecap]`; `idxpool = vec![0; n]`;
     `idxptr = 0`; `idxmax = 1`; store `maxn=n`, `alphsz=m`.

5. **`ChTrie::walk(from, sym, create) -> Option<usize>`** — depends on 3–4.
   - `h = (from.wrapping_mul(alphsz).wrapping_add(sym)) % ecap`
     (mirrors C unsigned wrap-then-mod).
   - Search `etab[h]` for an edge with matching `from` and `sym`; return
     `Some(to)` if found.
   - If `create`:
     - Capacity check: if `idxptr == 0 && idxmax >= maxn` → return `None`.
     - Allocate `to`: if `idxptr > 0 { to = idxpool[idxptr-1]; idxptr -= 1 }`
       else `{ to = idxmax; idxmax += 1 }`.
     - Head-insert `Edge { from, sym, to }` at `etab[h].insert(0, edge)`.
     - Return `Some(to)`.
   - Else return `None`.

6. **`ChTrie::delete(from, sym)`** — depends on 3–4.
   - `h = (from.wrapping_mul(alphsz).wrapping_add(sym)) % ecap`.
   - Find the matching edge in `etab[h]`; if present, remove it and push its
     `to` back onto the pool: `idxpool[idxptr] = to; idxptr += 1`.
   - If absent, leave the trie unchanged.

7. **`impl Drop for ChTrie`** — depends on 3.
   - Empty body (documented); `Vec` handles cleanup.

8. **Unit tests in `mod tests`** — depend on 4–6.
   - `new_regulates_zero`: `new(0,0)` succeeds; `new` with huge `n` returns
     `Err(Range)` on overflow.
   - `walk_create_and_lookup`: create an edge, then look it up without create;
     verify `Some`/`None` behavior.
   - `delete_recycles_index`: add then delete, verify index is recycled and a
     non-existent delete is a no-op.

## Part B — Test files (bottom-up dependency order)

All in `tests/integration.rs`. Depends on the completed Part A.

1. **`struct StringSet` + `StringSet::new()`** — depends on `ChTrie::new`.
   - Holds `tr: ChTrie`, `term: Vec<bool>`, `nchild: Vec<usize>` sized to `N`.

2. **`StringSet::add(&mut self, s: &str)`** — depends on `ChTrie::walk`.
   - Walk/create each byte symbol from node 0; mark `term` at the end;
     increment `nchild` for each created edge.

3. **`StringSet::del(&mut self, s: &str)`** — depends on `ChTrie::walk`,
   `ChTrie::delete`.
   - Walk the word; if present, clear `term` and prune leaf nodes (delete
   edges, decrement `nchild`) bottom-up.

4. **`StringSet::query(&self, s: &str) -> bool`** — depends on `ChTrie::walk`.
   - Walk without create; return `term` of the final node if the path exists.

5. **`#[test] fn string_set()`** — depends on 1–4.
   - Build the set from dict1/dict2/dict3, delete the stop list, then assert
     the 14 expected query results with `assert_eq!`.

## Build & Verify

- `cargo build` — library compiles.
- `cargo test` — unit tests (Part A step 8) and integration test (Part B step 5)
  pass.
