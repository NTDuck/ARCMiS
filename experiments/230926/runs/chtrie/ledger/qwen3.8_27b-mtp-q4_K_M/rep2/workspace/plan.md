# Translation Plan: CH-Trie (C -> Rust)

Strategy:
1. Use safe Rust only; no `unsafe` blocks.
2. Represent the edge table as `Vec<Vec<Edge>>` (one inner Vec per node), replacing the C flat array + capacity bookkeeping.
3. Keep the index pool (`idxpool`, `idxptr`, `idxmax`) as `Vec<usize>` + cursor for node id allocation.
4. Return `Result<_, ChTrieError>` from `walk`/`del` instead of C sentinel return values (NotFound, Range, Capacity).
5. Implement `Drop` for `ChTrie` to replace `chtrie_free`; `chtrie_alloc` becomes `ChTrie::new`.
6. Translate `tests/test.c` into Rust unit/integration tests using `cargo test`.
7. Verify with `cargo build` and `cargo test`; fix all errors and failures.
