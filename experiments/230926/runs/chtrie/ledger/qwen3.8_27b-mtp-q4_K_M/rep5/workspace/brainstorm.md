# CH-Trie C → Rust Port — Brainstorm

> Note: the workspace does not contain the C sources (`src/chtrie.c`,
> `src/chtrie.h`, `tests/test.c`, `Makefile`, `README.md`) described in the
> task; it already contains a Rust crate with a completed `src/lib.rs` port
> (see `notes.md`, session 1). The C semantics below are reconstructed from
> that port's line-by-line documentation and the standard CH-Trie code.

## 1. Core difficulties of the C → Rust translation

- **Manual memory management.** `chtrie_alloc` uses `calloc` for the struct,
  the `etab` slot array, and the `idxpool`; `chtrie_walk` `malloc`s one
  `chtrie_edge` per insert; `chtrie_del`/`chtrie_free` `free` them. In Rust
  this becomes ownership: `Vec<Vec<Edge>>` for the hash table, `Vec<i32>`
  for the pool, and a no-op `Drop` for API parity.
- **errno-based error signaling.** C signals failure via `errno = ERANGE`
  (size params too large) / `ENOMEM` (allocation failure) plus `NULL` or
  `-1` return. Rust should use `Result` with a small error enum instead of
  a thread-local errno.
- **Shared mutable struct with raw pointers.** `struct chtrie` holds
  `int *idxpool`, `int *idxptr` (a pointer *into* the pool acting as a
  stack pointer), `struct chtrie_edge **etab` (array of linked-list heads).
  Rust: `idxptr` becomes a `usize` offset into `idxpool`; `etab` becomes
  `Vec<Vec<Edge>>` where each slot's `Vec` is the linked list.
- **Index pool with idxptr/idxmax.** Pool is a stack: `del` pushes `to`
  at `*idxptr++`; `walk` pops `*--idxptr` when `idxptr > idxpool`, else
  allocates `idxmax++`. Exhaustion (`idxptr == idxpool && idxmax >= maxn`)
  is the ENOMEM case in `walk`. This must be preserved exactly — it is the
  observable behavior the tests depend on (deleted indices are reused in
  LIFO order).
- **C89 constraints.** Declarations at block tops, `unsigned long` hash
  arithmetic that wraps modulo 2^64 even for negative `from`/`sym`. Rust
  reproduces the wrap with `wrapping_mul`/`wrapping_add` on `usize` casts.
- **Test harness differences.** C test is a single `main()` with 14
  `assert()`s and `printf("ok\n")`. Rust: `#[cfg(test)]` unit tests and/or
  an integration test in `tests/`, using `assert_eq!`/`assert!`, run by
  `cargo test`.

## 2. Candidate approaches

1. **1:1 structural port (chosen).** Keep the same fields, same hash
   formula, same prepend-on-insert, same pool-stack semantics; only swap
   C memory primitives for Rust ownership. Minimal semantic risk; tests
   port verbatim.
2. **Idiomatic rewrite.** E.g. `HashMap<(i32,i32), i32>` for the table,
   `VecDeque`/`Vec` for the pool, `Option<Edge>` per slot. Cleaner Rust,
   but changes observable behavior (index-reuse order, hash-slot layout)
   and risks diverging from the C test expectations. Rejected.
3. **`unsafe` + raw pointers mirroring C exactly.** No benefit; the
   algorithm needs no FFI. Rejected.

## 3. Recommended approach

Approach 1: a faithful, safe, no-`unsafe` port.

API mapping:

| C | Rust | Notes |
|---|------|-------|
| `chtrie_alloc(n, m)` | `ChTrie::new(n, m) -> Result<ChTrie, ChTrieError>` | `Erange` for the two ERANGE checks (`n>m` INT_MAX bounds; `MIN(INT_MAX, SIZE_MAX) - (n-1) < (n-1)/3`), `Enomem` for allocation failure (in practice unreachable with `Vec`, but kept in the enum for parity). `n`, `m` regulated to ≥ 1; `ecap = (n-1) + (n-1)/3`; pool of `n` slots; `idxmax = 1`, `idxptr = 0`. |
| `chtrie_walk(tr, from, sym, creat)` | `walk(&mut self, from, sym, creat) -> i32` | Returns `to` if found; `-1` if absent and `!creat`; `-1` on pool exhaustion (C `errno = ENOMEM`). New edges prepended to the slot list (`Vec::insert(0, ..)`). |
| `chtrie_del(tr, from, sym)` | `del(&mut self, from, sym)` | Removes the edge, pushes `to` onto the pool stack at `idxptr`; no-op if absent. |
| `chtrie_free(tr)` | `impl Drop for ChTrie` | No-op; owned `Vec`s free their memory. |

Hash: `h = ((from as usize).wrapping_mul(alphsz as usize)
.wrapping_add(sym as usize)) % ecap` — reproduces C `unsigned long`
wraparound for negative inputs.

## 4. Test port (tests/test.c, 14 assertions)

The C test (n=100, m=10) and expected results:

1. `walk(0,0,creat=1) == 1` (fresh index 1)
2. `walk(0,0,creat=0) == 1` (lookup hit)
3. `walk(0,1,creat=1) == 2`
4. `walk(1,0,creat=1) == 3`
5. `walk(1,0,creat=0) == 3`
6. `walk(1,1,creat=0) == -1` (absent, no create)
7. `del(0,0)`; `walk(0,0,0) == -1` (deleted)
8. `walk(0,0,creat=1) == 1` (pooled index 1 reused)
9. `walk(0,1,0) == 2` (unaffected)
10. `walk(1,0,0) == 3` (unaffected)
11. `del(0,1)`; `walk(0,1,creat=1) == 2` (pooled index 2 reused)
12. `del(1,0)`; `walk(1,0,creat=1) == 3` (pooled index 3 reused)

Port to Rust:
- **Integration test** `tests/test.rs` mirroring the C test 1:1
  (`assert_eq!` for each of the 12 value assertions; the C file's 14
  `assert`s include the `chtrie_alloc` non-NULL check and the final
  `printf("ok")`, which map to `ChTrie::new(...).unwrap()` and test
  completion).
- **Unit tests** in `src/lib.rs` under `#[cfg(test)]` for edge cases the
  C test doesn't cover: `new(0, 0)` regulation, `new` ERANGE with huge
  `n`, `del` of a nonexistent edge, pool exhaustion (`new(2, 1)` then
  create beyond `maxn` → `-1`).

## 5. Build setup (Makefile / runtest → Cargo)

The C `Makefile` (`make test` builds `test` and runs it) and any `runtest`
script are replaced by `cargo test`. `Cargo.toml` (already present):

```toml
[package]
name = "chtrie"
version = "0.1.0"
edition = "2021"

[lib]
name = "chtrie"
path = "src/lib.rs"

[workspace]   # keep out of any enclosing workspace
```

No external dependencies (no `rand`, no `libc`); `std` only.
