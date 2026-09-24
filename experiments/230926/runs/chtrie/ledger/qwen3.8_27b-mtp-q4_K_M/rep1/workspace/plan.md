# CH-Trie C -> Rust Translation Plan

## Strategy

Translate `src/chtrie.c` + `src/chtrie.h` into a Rust crate:

- `Cargo.toml` — crate manifest (name `chtrie`, edition 2021).
- `src/lib.rs` — re-export the public API from `chtrie` module.
- `src/chtrie.rs` — core implementation.

## Public API

| Rust | C equivalent | Notes |
|------|-------------|-------|
| `ChTrie::new(n, m) -> Result<Self, ChTrieError>` | `chtrie_alloc` | clamps n,m >= 1; overflow guards |
| `ChTrie::walk(&mut self, from, sym, creat) -> Result<usize, ChTrieError>` | `chtrie_walk` | NotFound when creat=false and missing; Capacity when exhausted |
| `ChTrie::del(&mut self, from, sym)` | `chtrie_del` | no-op on missing edge |
| `Drop for ChTrie` | `chtrie_free` | recursive cleanup |

## Invariants to Preserve

- `ecap = (n-1) + (n-1)/3`
- Hash: `h = (from * alphsz + sym) % ecap`
- LIFO index pool: `idxpool` stack + `next_idx` counter
- Clamping: n, m >= 1
- Overflow guards: n or m > i32::MAX -> `Range`; (n-1)+(n-1)/3 > i32::MAX -> `Range`
- `del` is a no-op on missing edge
- Root index 0 is never placed in the pool

## Tests

Port `tests/test.c` into a `#[cfg(test)]` module:

- `StringSet` helper: `ChTrie` + `term: Vec<bool>` + `nchild: Vec<usize>`
- One `#[test]` running the 14 query assertions:
  hello=0, the=0, his=1, he=1, his=1, go=0, he=1, a=0, an=0, this=1, that=1, hey=0, she=1, hers=1
- Extra unit tests:
  - `new(0,0)` clamping
  - `new` overflow -> `Range`
  - `walk` creat=0 miss -> `NotFound`
  - `del` nonexistent edge no-op
  - LIFO index reuse after `del`

## Build

```
cargo test
```
