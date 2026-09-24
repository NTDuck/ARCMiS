# chtrie (C)

CH-Trie C source — the reference implementation for the Rust translation.

- `src/chtrie.h` / `src/chtrie.c` — the trie library (alloc, walk, del, free).
- `tests/test.c` — test driver (add/del/query over small dictionaries).
- `Makefile` — build (`make all`), test (`make test`), install, clean.

Build and test:

```sh
make test
```
