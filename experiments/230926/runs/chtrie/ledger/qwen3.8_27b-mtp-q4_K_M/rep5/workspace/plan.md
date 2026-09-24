# CH-Trie Rust Port — Plan

## Strategy
Idiomatic Rust port of the C CH-Trie library:
- **Owned `Vec`s** for `etab` (hash slots, each a `Vec<Edge>` list) and `idxpool` (fixed-size index pool, used as a stack at `idxptr`).
- **`Result<ChTrie, ChTrieError>`** for allocation (`new`), mirroring `errno = ERANGE/ENOMEM` in C.
- **`&mut self` methods** for `walk` and `del`.
- **`Drop` impl** for API parity (no-op, since `Vec`s free their memory automatically).
- **No `unsafe`** anywhere; C unsigned-wrap hash arithmetic reproduced with `wrapping_mul`/`wrapping_add`.
- Same hash formula `h = (from * alphsz + sym) % ecap`, same prepend-on-insert, same stack semantics for the index pool.

## Task List
1. ~~Brainstorm / design the model~~ (done)
2. Create crate (`Cargo.toml`) and port `chtrie.c` to `src/lib.rs` (in progress)
3. Port tests (pending)
4. Build + test verification (pending)
