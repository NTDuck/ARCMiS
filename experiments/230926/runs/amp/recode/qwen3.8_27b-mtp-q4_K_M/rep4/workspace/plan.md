# AMP — C → Rust Implementation Plan

Source: C clibs `amp` package (see `design.md`). Target: std-only Rust cargo
package `amp`, edition 2021, no external crates. Test command: `cargo test`.

Skeleton files already exist and compile: `Cargo.toml`, `src/lib.rs`,
`tests/test.rs`. Each stub below is marked with `TODO` / `unimplemented!` —
replace the stub body with the implementation described, in order.

## Name mapping (C → Rust)

| C symbol | Rust symbol | Notes |
|---|---|---|
| `AMP_VERSION` (macro, `src/amp.h`) | `pub const VERSION: u8` | `AMP_` prefix dropped (Rust items are already in the `amp` crate namespace) |
| `amp_t` (struct) | `pub struct AmpMessage<'a>` | C struct + cursor pointer becomes a generic struct borrowing the buffer |
| `amp_t.version` (`short`) | `AmpMessage.version: u8` | values fit in a byte |
| `amp_t.argc` (`short`) | `AmpMessage.argc: u8` | 0–15 |
| `amp_t.buf` (`char *`) | `AmpMessage.buf: &'a [u8]` (private) | cursor slice, zero-copy |
| `amp_encode(char **argv, int argc)` | `pub fn encode(argv: &[&[u8]]) -> Result<Vec<u8>, AmpError>` | NULL-on-OOM → `Result`; adds `TooManyArgs` validation |
| `amp_decode(amp_t *msg, char *buf)` | `AmpMessage::new(buf: &'a [u8]) -> Result<Self, AmpError>` | out-param struct → constructor returning `Result` |
| `amp_decode_arg(amp_t *msg)` | `AmpMessage::decode_arg(&mut self) -> Result<&'a [u8], AmpError>` | malloc'd copy + `free` → borrowed slice; advances cursor |
| `read_u32_be` / `write_u32_be` (static) | — (no stub) | replaced by `u32::from_be_bytes` / `u32::to_be_bytes` |
| `NULL` returns / UB on truncation | `AmpError` enum: `EmptyBuffer`, `TooManyArgs`, `TruncatedArgument`, `ArgMismatch` | new type, no C counterpart |
| `tests/test.c` `main` | `tests/test.rs` `round_trip` | `assert`/`strcmp` → `assert_eq!` on byte slices |

## Part A — source files (bottom-up dependency order)

### A1. `Cargo.toml` (done — skeleton in place)
- `[package]` name `amp`, version `0.0.1`, edition `2021`, description,
  license MIT.
- Empty `[workspace]` table so the package stays standalone when nested in
  another cargo workspace.
- No `[dependencies]` (std only).
- Verify: `cargo check` passes.

### A2. `src/lib.rs` (single file; implement items in this order)

1. **`pub const VERSION: u8 = 1`** — done in skeleton.
2. **`pub enum AmpError`** — done in skeleton
   (`EmptyBuffer`, `TooManyArgs`, `TruncatedArgument`, `ArgMismatch`,
   `#[derive(Debug, PartialEq)]`). No further work.
3. **`AmpMessage::new(buf: &'a [u8]) -> Result<Self, AmpError>`**
   (replaces `amp_decode`):
   - `buf.is_empty()` → `Err(AmpError::EmptyBuffer)`.
   - `version = buf[0] >> 4`, `argc = buf[0] & 0x0f`, cursor `buf = &buf[1..]`.
   - Returns `Ok(Self { version, argc, buf })`.
4. **`AmpMessage::decode_arg(&mut self) -> Result<&'a [u8], AmpError>`**
   (replaces `amp_decode_arg`):
   - If no arguments remain (`argc` exhausted) → `Err(AmpError::ArgMismatch)`.
   - If `< 4` bytes left → `Err(AmpError::TruncatedArgument)`.
   - `len = u32::from_be_bytes(buf[0..4].try_into().unwrap()) as usize`.
   - If `len > buf.len() - 4` → `Err(AmpError::TruncatedArgument)`.
   - `let arg = &buf[4..4 + len]; self.buf = &buf[4 + len..];` decrement
     remaining-arg counter; return `Ok(arg)`.
   - Note: track remaining args (e.g. a private `remaining: u8` field set in
     `new`, or derive from `argc` vs. decoded count) to implement
     `ArgMismatch`.
5. **`pub fn encode(argv: &[&[u8]]) -> Result<Vec<u8>, AmpError>`**
   (replaces `amp_encode`):
   - `argv.len() > 15` → `Err(AmpError::TooManyArgs)`.
   - `let mut out = Vec::with_capacity(1 + argv.iter().map(|a| 4 + a.len()).sum::<usize>())`.
   - Push header byte `(VERSION << 4) | argv.len() as u8`.
   - Per arg: push `len as u32` via `u32::to_be_bytes`, then the raw bytes.
   - Return `Ok(out)`.
- Verify: `cargo build` and `cargo check --all-targets` pass with no errors.

## Part B — test files (bottom-up dependency order)

### B1. `tests/test.rs` (depends on A2; implement tests in this order)

1. **`round_trip`** — direct port of `tests/test.c` `main`:
   encode `[b"some", b"stuff", b"here"]`, assert `version == 1`,
   `argc == 3`, then `decode_arg` three times asserting
   `b"some"`, `b"stuff"`, `b"here"`.
2. **`truncated_buffer`** — encode one arg, truncate the buffer mid-argument,
   assert `decode_arg` (or `new` on a 0-byte buffer) returns
   `Err(AmpError::TruncatedArgument)` / `Err(AmpError::EmptyBuffer)`.
3. **`too_many_args`** — 16 one-byte args; assert
   `encode(...) == Err(AmpError::TooManyArgs)`.
4. **`empty_argv`** — `encode(&[])` yields exactly one byte `0x10`;
   `AmpMessage::new` on it gives `version == 1`, `argc == 0`.
5. **`embedded_nul_bytes`** — encode `[b"a\0b"]`, decode, assert the arg
   equals `b"a\0b"` (length-prefixed format is NUL-safe).
- Verify: `cargo test` — all 5 tests pass.

## Definition of done
- `cargo build` succeeds with no errors.
- `cargo test` runs `tests/test.rs` and all tests pass.
- No external crates in `Cargo.toml`.
