# Notes

## Brainstorm (pre-code)

### Core difficulties

1. **Non-standard MurmurHash3 variant — reverse chunk order.**
   The C chunk loop is `for (i = -l; i != 0; ++i)` over `chunks = d + l*4`, i.e. it
   walks the 4-byte chunks from the LAST chunk to the FIRST. Standard MurmurHash3
   goes forward. The Rust port must reproduce the exact C behavior, not the
   standard algorithm. Acceptance: C test vectors must match, e.g.
   `"kinkajou"` seed 0 -> `0xb6d99cf8`, `""` seed 1 -> `0x514e28b7`.
   - Candidate approaches:
     a. Direct translation: `for i in (0..l).rev()` indexing `chunks[l-1-i]` —
        closest to the C, easiest to verify line-by-line. (Preferred.)
     b. Iterate a slice in reverse: `chunks.iter().rev()` — idiomatic, same result.
     c. "Fix" it to standard MurmurHash3 — REJECTED, breaks the vectors.

2. **Remainder switch with fallthrough.**
   C uses `case 3: ... case 2: ... case 1: ...` with no `break`s, so a 3-byte
   remainder executes the case-3, case-2, and case-1 bodies in sequence.
   - Candidate approaches:
     a. `match len % 4 { 3 | 2 | 1 => ... }` with the bodies inlined in the right
        order per arm (arm 3 contains all three bodies, arm 2 the last two).
     b. `if len % 4 >= 3 { ... } if len % 4 >= 2 { ... } if len % 4 >= 1 { ... }`
        — mirrors the fallthrough literally. (Preferred for fidelity.)

3. **Endianness / htole32.**
   `htole32` is a no-op on little-endian, byte-swap on big-endian.
   - Candidate: `u32::from_le_bytes` for loads, `u32::to_le_bytes` for stores.
     Portable and matches C semantics on both endians.

4. **Overflow semantics.**
   C `uint32_t` arithmetic wraps silently; Rust debug builds panic on overflow.
   - Candidate: use `wrapping_mul` / `wrapping_add` (or `overflowing_*`) for the
     mix operations so `cargo test` (debug profile) behaves like C.

5. **main.c CLI bugs.**
   - Malformed `printf("%d" PRIu32, ...)` — format string is broken in C.
   - The loop re-hashes `buf` instead of `key`, so it prints the same hash
     repeatedly.
   - Decision: do NOT translate the bugs. Produce a working CLI: read stdin
     lines, hash each line with an optional `--seed N` (default 0), print the
     decimal hash per line.

6. **Input handling details.**
   - C `strlen` semantics: hash the bytes up to NUL; Rust `&[u8]` slice length
     is explicit — use the line bytes as-is (strip trailing newline in the CLI).
   - Empty input must still run the finalization (mix) phase — covered by the
     `""` seed 1 vector.

### Candidate overall structure
- `Cargo.toml` — package `murmurhash`, `[lib]` + `[[bin]]`, no deps.
- `src/lib.rs` — `pub fn murmurhash3(data: &[u8], seed: u32) -> u32` + tests
  ported from `test.c`.
- `src/main.rs` — CLI (stdin lines, `--seed`, decimal output).

### Verification plan
- `cargo test` must pass with the exact C vectors.
- Manual CLI check: `printf 'kinkajou\n' | cargo run` -> decimal of `0xb6d99cf8`
  (= 3016867736).
