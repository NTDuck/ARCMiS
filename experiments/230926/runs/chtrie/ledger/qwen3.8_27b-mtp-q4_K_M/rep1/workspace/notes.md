# Brainstorm Findings

- **C self-referential edge list** -> use `Vec<Option<Box<Edge>>>` or `Vec<Vec<Edge>>` owned by `ChTrie`; `Drop` replaces `chtrie_free`.
- **errno** -> typed `ChTrieError` enum `{ Range, Capacity, Alloc, NotFound }`.
- **Index pool**: `Vec<usize>` LIFO stack + `next_idx`; `del` pushes child index, `walk(creat)` pops if non-empty else `next_idx++`.
- **Overflow guard** simplifies to `(n-1)+(n-1)/3 > i32::MAX -> Range`.
- **test.out is stale** (15 lines, matches old `test.c_old`); authoritative expectations are the 14 in `tests/test.c`.
- **C bug**: `walk` malloc-failure path doesn't set errno; Rust returns `Err(Alloc)` consistently.
- **add()** increments `nchild` only when edge didn't exist (probe `creat=0` first); `del` prunes non-terminal zero-child chain.
- **Empty string add** sets `term[0]=1`.

---

## Work Log

- Created `plan.md` (translation strategy, API table, invariants, test plan).
- Created `tasks.json` (5 seed tasks t1–t5, valid JSON).
- Created `notes.md` (brainstorm findings).

## Work Log (continued)

- Wrote `brainstorm.md`: core difficulties (pointers/ownership, struct layout,
  errno→Result, C89 vs idioms, test porting, `del` LIFO pool semantics),
  three candidate approaches (A direct port / B HashMap rewrite / C hybrid),
  recommended A/C, crate layout + public API, and a 12-item risk/edge-case table.

## Audit Log

- Ran full audit: listed files, read plan.md / notes.md / brainstorm.md /
  Cargo.toml / tasks.json / src/lib.rs / src/chtrie.rs.
- `cargo build`: OK (cached, no errors).
- `cargo test`: 6 passed, 1 FAILED — `chtrie::tests::test_new_clamping`
  panics: `new(0,0)` clamps to `maxn=1`, so `walk(0,0,true)` returns
  `Err(Capacity)` (next_idx=1 >= maxn=1) instead of allocating node 1.
  The test expects the clamped trie to still be able to create one node.
  Root cause: clamp-to-1 leaves zero allocatable nodes; either the test
  expectation or the clamp semantics need revisiting (C behavior: with
  n=1 the index space is exhausted after the root).

## Fix: test_new_clamping (C-faithful expectation)

- The failing test expected `walk(0, 0, true)` on a `ChTrie::new(0, 0)` trie
  to allocate node 1. That expectation was wrong.
- C-faithful behavior (src/chtrie.c): `chtrie_new` clamps n and m to >= 1,
  so n=1 leaves only the root node (index 0). `chtrie_walk(tr, 0, sym, 1)`
  then hits `tr->idxptr == tr->idxpool && tr->idxmax >= tr->maxn` and
  returns -1 with `errno = ENOMEM`.
- The Rust implementation already mirrors this: `walk` returns
  `Err(ChTrieError::Capacity)` (the ENOMEM analog). Implementation of
  walk/new/del was NOT changed.
- Updated `test_new_clamping` to assert: `new(0,0)` succeeds (clamped),
  `walk(0, 0, true) == Err(ChTrieError::Capacity)`, and `new(0,5)` /
  `new(5,0)` still succeed.
- Result: `cargo build` OK; `cargo test` 7 passed, 0 failed.

## Final verification (ledger worker)
- `cargo build --locked`: exit 0, no warnings (forced recompile via touch).
- `cargo test --locked`: exit 0, 7 passed / 0 failed (test_del_noop, test_capacity, test_index_reuse_lifo, test_new_clamping, test_new_overflow, test_walk_miss, test_queries); 0 doc-tests.
- API confirmed in src/chtrie.rs: ChTrie::new(n,m)->Result<Self,ChTrieError>, walk(&mut,from,sym,creat)->Result<usize,ChTrieError>, del(&mut,from,sym); ChTrieError::{Range,Capacity,Alloc,NotFound}; pub struct Edge, pub struct ChTrie.
- test_queries: 14 queries, expected bools false,false,true,true,true,false,true,false,false,true,true,false,true,true == C expected {0,0,1,1,1,0,1,0,0,1,1,0,1,1}. Match.
