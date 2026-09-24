# Plan: C → Rust translation of CH-Trie

## Cargo project layout

```
Cargo.toml          # package "chtrie", edition 2021, lib name "chtrie"
src/lib.rs          # the translated chtrie (ChTrie, Edge, ChTrieError)
tests/test.rs       # translated tests/test.c (14 query assertions)
```

## API (faithful mirror of the C header)

- `ChTrie::alloc(n: usize, m: usize) -> Result<ChTrie, ChTrieError>`
  - `Err(ChTrieError::Range)` for the C `ERANGE` cases (`n`/`m` beyond `i32::MAX`,
    or `ecap = (n-1) + (n-1)/3` overflowing `i32::MAX`).
- `walk(&mut self, from: i32, sym: i32, creat: bool) -> Result<Option<i32>, ChTrieError>`
  - `Ok(Some(idx))` — child found or created;
  - `Ok(None)` — child not found (C `-1` in the non-error case);
  - `Err(ChTrieError::Capacity)` — `creat` set but node pool exhausted (C `ENOMEM`).
- `del(&mut self, from: i32, sym: i32)` — delete a leaf edge; no-op if absent.
- Memory freed by `Drop` (replaces `chtrie_free`); no manual free needed.

## Internal state (private fields)

- `etab: Vec<Option<Box<Edge>>>` — hash buckets, each the head of a singly-linked
  edge list; `Edge { next: Option<Box<Edge>>, from: i32, sym: i32, to: i32 }`.
- `idxpool: Vec<i32>` — pool of free node indexes (size `n`).
- `idxptr: usize` — cursor into `idxpool` (replaces the C pointer; `0` = pool empty).
- `idxmax: i32` — next fresh node index (starts at 1; 0 is the root).
- `maxn: usize` — maximum node count.
- `alphsz: usize` — alphabet size.
- `ecap: usize` — hash-table capacity, `(n-1) + (n-1)/3`.

Hash: `h = (from * alphsz + sym) % ecap` using wrapping unsigned arithmetic to mirror
C's `unsigned long` wraparound.

## Verification

- `cargo build` succeeds.
- `cargo test` passes all 14 query assertions:
  - build trie with `N = 65536`, `M = 256`;
  - add dict1 = `["", "the", "a", "an"]`, add dict2 = `["he", "she", "his", "hers"]`;
  - del stop = `["the", "an", "a"]`;
  - add dict3 = `["this", "that"]`;
  - queries `hello the his he his go he a an this that hey she hers`
    must equal `0 0 1 1 1 0 1 0 0 1 1 0 1 1`.
- Extra unit tests for the error paths: `alloc` with `n`/`m` beyond `i32::MAX`
  → `Err(ChTrieError::Range)`; pool exhaustion → `Err(ChTrieError::Capacity)`.
