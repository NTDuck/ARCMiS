# C -> Rust Translation Plan (murmurhash)

## Goal
Port the non-standard MurmurHash3 variant (`murmurhash.c`) to Rust, keeping hash
values bit-identical to the C test vectors (e.g. `"kinkajou"` seed 0 -> `0xb6d99cf8`,
`""` seed 1 -> `0x514e28b7`). Target test command: `cargo test`.

## Structure
- `Cargo.toml` — package `murmurhash`, lib + bin targets.
- `src/lib.rs` — `pub fn murmurhash3(data: &[u8], seed: u32) -> u32` plus `#[cfg(test)]`
  tests ported from `test.c`.
- `src/main.rs` — working CLI: reads stdin lines, hashes each with optional `--seed N`,
  prints the decimal hash.

## Steps
1. Translate the core hash function:
   - `htole32` -> `u32::from_le_bytes` / `u32::to_le` (no-op on little-endian, correct
     on big-endian).
   - Chunk loop `for (i = -l; i != 0; ++i)` over `chunks = d + l*4` — chunks are
     processed in REVERSE order (last chunk first). Preserve exactly; do NOT "fix"
     to standard MurmurHash3.
   - Remainder switch with fallthrough (case 3 -> case 2 -> case 1) -> `match` with
     explicit combined arms (3, 2, 1) or an if-chain; keep the same operations.
   - Use `u32` wrapping arithmetic (Rust debug builds panic on overflow — use
     `wrapping_mul` / `wrapping_add` or `debug_assertions = false`; prefer explicit
     wrapping ops for clarity).
2. Port `test.c` vectors into `#[test]` functions.
3. Write the CLI (`main.rs`) as a working tool (the C `main.c` has bugs: malformed
   `printf "%d" PRIu32`, loop re-hashes `buf` instead of `key`).
4. `cargo build`, `cargo test`, fix until green.

## Risks / Notes
- Reverse chunk order is the whole point of this variant — any "normalization"
  breaks the vectors.
- Overflow semantics: C wraps, Rust debug builds panic.
- Endianness: use `to_le`/`from_le` rather than assuming little-endian.
