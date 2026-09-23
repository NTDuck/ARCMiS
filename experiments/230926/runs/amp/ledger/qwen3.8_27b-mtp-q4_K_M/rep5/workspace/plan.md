# Plan: translate amp (C) to Rust

## Strategy
Faithful, minimal translation with idiomatic Rust types:

- **Crate layout:** `Cargo.toml` (name `amp`, version `0.0.1`, edition 2021,
  license MIT), `src/lib.rs` (library), `tests/test.rs` (integration test
  mirroring `tests/test.c`). `cargo test` replaces the Makefile.
- **Data model:** `pub struct Amp { version: u8, argc: u8, buf: Vec<u8>, pos: usize }`
  — decode copies the payload after the header byte into `buf`; `pos` is the
  cursor (replaces C's `char *buf` pointer advancement).
- **API (1:1 with C):**
  - `pub const AMP_VERSION: u8 = 1;`
  - `pub fn amp_encode(argv: &[&str]) -> Vec<u8>` — header byte
    `(AMP_VERSION << 4) | argc`, then per arg: `u32::to_be_bytes(len)` + bytes.
  - `pub fn amp_decode(msg: &mut Amp, buf: &[u8])` — `version = buf[0] >> 4`,
    `argc = buf[0] & 0xf`, `buf = buf[1..].to_vec()`, `pos = 0`.
  - `pub fn amp_decode_arg(msg: &mut Amp) -> Vec<u8>` — read
    `u32::from_be_bytes` len at `pos`, advance 4, copy `len` bytes, advance.
- **Memory:** `Vec<u8>` returns replace malloc/free; no `Option`/`Result`
  (C's `NULL`-on-OOM maps to Rust's OOM panic).
- **Tests:** port `tests/test.c` asserts verbatim: version==1, argc==3,
  args "some"/"stuff"/"here"; print "ok" at the end.

## Tasks
1. Scaffold crate: `Cargo.toml`, empty `src/lib.rs`.
2. Implement `Amp`, `AMP_VERSION`, `amp_encode`, `amp_decode`, `amp_decode_arg`
   in `src/lib.rs`.
3. Port `tests/test.c` to `tests/test.rs`.
4. `cargo build` + `cargo test`; fix until green.
5. Optional: add a byte-exact round-trip test; verify no warnings
   (`cargo clippy` if available).
