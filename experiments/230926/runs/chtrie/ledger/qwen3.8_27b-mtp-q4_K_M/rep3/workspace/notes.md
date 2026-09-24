# notes.md

## Findings (history)

- **2025-06-15 (ledger worker):** Workspace contained only `notes.md`,
  `plan.md`, `tasks.json` — the entire C source tree was missing.
  Restored byte-faithful: `src/chtrie.h`, `src/chtrie.c`, `tests/test.c`,
  `Makefile`, and created `README.md` (was missing).
- **C verification (gcc 15.3.0):**
  - `cc -c -o /tmp/chtrie.o src/chtrie.c` → OK (no warnings).
  - `cc -o /tmp/run.tmp src/chtrie.c tests/test.c` → fails by default:
    `tests/test.c` uses `exit` without `#include <stdlib.h>` (as given in
    the spec), which GCC 14+ treats as an error (implicit function
    declaration). File kept byte-faithful per task; compiled with
    `-Wno-implicit-function-declaration` instead.
  - `/tmp/run.tmp` → all 14 queries match expected results, prints
    "All tests passed!".

## 2025-06-15 (ledger worker) — re-planning against the real source

Re-inspected `src/chtrie.c`, `src/chtrie.h`, `tests/test.c`, `Makefile`
and rewrote `plan.md` + `tasks.json`. The previous plan (written before
the source was restored) had two factual errors, now corrected:

1. **Edge table is GLOBAL, not per-node.** `etab` is one flat array of
   `ecap` buckets; each bucket is a linked list of edges `{from, sym, to}`
   keyed by the pair `(from, sym)`, hashed `h = (from*alphsz + sym) % ecap`.
   Old plan said "per-node edge hash table" — wrong.
2. **`ERANGE` is an alloc-time size/overflow guard, not a symbol-range
   error.** Set by `chtrie_alloc` when `n > INT_MAX`, `m > INT_MAX`, or
   `INT_MAX - (n-1) < (n-1)/3`. `ENOMEM` is set by `chtrie_walk` when the
   index pool is exhausted. Old plan's `ChTrieError::SymbolRange` is
   dropped; new variants: `ChTrieError::{Range, Capacity}`.

### Core difficulties identified (with chosen approach)

- **errno/NULL error returns vs Rust `Result`.**
  C mixes "not found" (`-1`, no errno) with "cannot create" (`-1` +
  `ENOMEM`) in one return code. → `walk` returns
  `Result<Option<i32>, ChTrieError>`: `Ok(None)` = not found,
  `Err(Capacity)` = pool exhausted; `alloc` returns
  `Result<ChTrie, _>` with `Err(Range)` for the ERANGE conditions.
- **Raw-pointer struct layout of `chtrie`.**
  `etab` (array of `struct chtrie_edge *`), `idxpool`/`idxptr` (pointer
  arithmetic into the pool), `idxmax`. → Own everything in `ChTrie` with
  `Vec`s: `etab: Vec<Option<Vec<Edge>>>` (or `Vec<Option<Edge>>` buckets),
  `idxpool: Vec<i32>` + `idxptr: usize` + `idxmax: i32`. No `Box`, no
  `unsafe`, no raw pointers; `Drop` replaces `chtrie_free`.
- **Index pool recycling.**
  `del` pushes the child index back (`*idxptr++ = p->to`); `walk(creat)`
  pops (`*--idxptr`) or allocates a fresh one (`idxmax++`); exhaustion is
  `idxptr == idxpool && idxmax >= maxn`. → `Vec<i32>` free list + counter;
  exact same pop/push order so index reuse is observable-identical.
  Note: the C code stores **no per-node data** — nodes are bare indices;
  do not invent a `Vec<Node>` arena.
- **Hash table with linked-list buckets.**
  → Per-bucket `Vec<Edge>` (prepend = `insert(0, ..)` to keep C's
  head-insertion order) or an edge arena with `next` indices; prefer the
  simple per-bucket `Vec`. Hash function and `ecap` formula preserved.
- **Test's static arrays `term[N]`/`nchild[N]` (and `nodes[N]`/`symbs[N]`)
  of size 65536.**
  → `Vec<i32>` of length `N` (65536 ints is trivial); `del`'s path trace
  becomes local `Vec`s. No C-style statics needed.
- **assert-based test style + `fatal`/`exit` on alloc failure.**
  → `#[test]` function in `tests/test.rs`; `assert_eq!(query(s), expected)`
  for all 14 cases (none weakened/deleted); `ChTrie::alloc` failure
  handled with `expect`/`unwrap` in the test (it must not fail for
  N=65536, M=256).
- **C89/C99 constraints, `size_t` vs `int` mixing, `INT_MAX` overflow
  guard.**
  → Rust `usize` for `n`/`m` in `alloc`, `i32` for node indices and
  symbols (matching C `int`); overflow check ported as
  `n > i32::MAX as usize || m > i32::MAX as usize || (n-1) + (n-1)/3`
  overflow → `Err(Range)`.
- **C quirk:** `chtrie_walk` returns `-1` on `malloc` failure *without*
  setting errno. → In Rust, `Vec` growth aborts on OOM; the only
  reachable error path is pool exhaustion → `Err(Capacity)`. Documented.

### Chosen approach (summary)

- Single crate `chtrie`: `Cargo.toml` (edition 2021, no deps),
  `src/lib.rs`, `tests/test.rs`. `make test` → `cargo test`.
- API: `ChTrie::alloc(n, m) -> Result<ChTrie, ChTrieError>`;
  `walk(&mut self, from: i32, sym: i32, creat: bool) ->
  Result<Option<i32>, ChTrieError>`; `del(&mut self, from: i32, sym: i32)`;
  `Drop`; `pub const ROOT: i32 = 0`;
  `enum ChTrieError { Range, Capacity }`.
- Zero `unsafe`; `Send`/`Sync` by construction.
- Tests: 1:1 port of `tests/test.c` (same dicts, same 14 assertions) in
  `tests/test.rs`; extra edge-case tests (Range, Capacity, recycling) in a
  `#[cfg(test)]` module in `lib.rs`.

### Risks / open items

- Borrow-checker friction in `walk`/`del` if a bucket `Vec` is borrowed
  while the pool mutates — avoid by copying out the found `to` index
  before any mutation (no overlapping borrows expected).
- `del` in the C test prunes only while `!term[it] && nchild[it] == 0`;
  the port must keep that exact pruning condition.
- `add` in the C test increments `nchild[it]` when the edge is absent
  *before* creating it — preserve that order.

## Verification round (ledger worker, continuation)

**Fix completed:** `ChTrie::alloc` now delegates size validation to a new
`ChTrie::validate(n, m) -> Result<(i32, i32, i32), ChTrieError>` helper
(clamp to >=1, `n/m > i32::MAX` check, overflow guard
`i32::MAX - (n-1) < (n-1)/3` mirroring C's
`MIN(INT_MAX, SZ_MAX) - (n-1) < (n-1)/3`) that runs **before any
allocation**. The `alloc_range_errors` unit test now exercises the guard
boundary through `validate` only (just-past guard `n = 1_610_612_737`
→ `Err(Range)`; just-under guard `n = 1_610_612_736` →
`Ok((1_610_612_736, 1, 2_147_483_646))`), so no test allocates the
~51 GB edge table. A small real allocation (`n = 1024, m = 256`) covers
the success path. Also added `#[allow(clippy::too_many_arguments)]` on
`walk` (faithful port of `chtrie_walk(tr, from, sym, creat)`).

**Final results (verbatim, workspace root):**

`cargo build`:
```
Compiling chtrie v0.1.0 (/home/ayin/projs/ARCMiS/experiments/230926/runs/chtrie/ledger/qwen3.8_27b-mtp-q4_K_M/rep3/workspace)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
```
(exit 0)

`cargo test`:
```
running 6 tests
test tests::alloc_clamps_to_one ... ok
test tests::alloc_range_errors ... ok
test tests::del_recycles_index ... ok
test tests::degenerate_single_node_trie ... ok
test tests::walk_capacity_on_pool_exhaustion ... ok
test tests::walk_finds_existing_edge ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 1 test
test chtrie_test ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
(exit 0)

`cargo clippy --all-targets`: clean (no warnings).

**Allocation bound:** the largest real allocation in any test is
`tests/test.rs` with N=65536, M=256 → `ecap = 65535 + 21845 = 87380`
buckets (a few MB at most); the guard-boundary test allocates nothing.
