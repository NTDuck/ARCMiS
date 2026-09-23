# ULID C → Rust Translation Plan

## Chosen approach: std-only (no external crates)

Rationale:
- The target command is `cargo test`, which must build **offline**. External crates
  (`getrandom`, `clap`) require network access to crates.io unless vendored;
  `std` alone always builds.
- Everything the C program does is available in `std`:
  - Entropy: read `/dev/urandom` (Linux) via `std::fs::File` — equivalent to `getentropy`.
  - Time: `SystemTime::now().duration_since(UNIX_EPOCH)` for the 48-bit ms timestamp.
  - Sleep fallback: `thread::sleep(Duration::from_millis(1))`.
  - CLI: manual `std::env::args()` parsing for `-n N` and `-t`.
  - Line-tagging mode: `BufRead::lines()` over stdin (replaces `getdelim`).

## Core difficulties identified

1. **Entropy source** — C uses `getentropy(16)`. Rust std has no direct equivalent;
   read 16 bytes from `/dev/urandom`. Handle read-short/error cases (C code likely
   ignores errors; Rust must decide: panic or retry).
2. **Shared state across calls** — `ulidgen_r` implements "same millisecond →
   increment the random part in place" by relying on the *caller reusing the same
   output buffer*. In Rust, model this as an explicit `UlidGen` struct holding the
   last timestamp (u64) and last random part (u128 or [u8;10]); `next(&mut self)`
   returns a fresh ULID. This is actually cleaner than C and makes the state
   ownership explicit.
3. **nanosleep + recursion fallback** — when the random part overflows (all bits
   set) within the same ms, C sleeps 1ms and recurses. In Rust: `thread::sleep`
   then loop (avoid unbounded recursion; a bounded loop or `loop` is safer).
4. **CLI parsing & line-tagging** — `-n N` (generate N ULIDs) and `-t` (tag each
   stdin line with a ULID). `getdelim` reads arbitrary-length lines; Rust
   `BufRead::lines()` handles this. Preserve line endings / handle final line
   without newline.
5. **Test translation** — C tests: length (26 chars), structure (Crockford
   base32 alphabet, valid timestamp decode), uniqueness (N distinct), sortability
   (lexicographic order == generation order). Translate 1:1 into `#[test]`
   functions in `tests/` (integration) or `#[cfg(test)]` in lib.
6. **Project layout** — `Cargo.toml` with a lib target (`src/lib.rs` for
   `UlidGen`, encoding/decoding) and a bin target (`src/main.rs` for the CLI),
   integration tests in `tests/test.rs`.

## File layout

```
Cargo.toml
src/lib.rs        # UlidGen struct, next(), encode/decode, entropy, time
src/main.rs       # CLI: -n N, -t line tagging
tests/test.rs     # length, structure, uniqueness, sortability
```

## Notes
- The C source files were **not present** in the workspace at brainstorm time;
  this plan is based on the task description of the C behavior. If the sources
  appear, verify details (exact overflow handling, error behavior) before coding.
