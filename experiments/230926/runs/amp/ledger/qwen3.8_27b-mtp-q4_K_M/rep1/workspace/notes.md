# Ledger notes

## Implementation

Translated the C AMP library into a Rust cargo project (`amp`, edition 2021, no dependencies).

### API chosen

- `pub const AMP_VERSION: u8 = 1;` — mirrors `#define AMP_VERSION 1`.
- `pub struct AmpMsg<'a> { pub version: u8, pub argc: u8, buf: &'a [u8], pos: usize }` — decoded header plus a cursor (`pos`) over the argument payload, replacing the C `char *buf` pointer that `amp_decode_arg` advances.
- `pub fn amp_encode(argv: &[&str]) -> Vec<u8>` — writes the header byte `AMP_VERSION << 4 | argc`, then for each arg a 4-byte big-endian length followed by the arg bytes.
- `pub fn amp_decode(buf: &[u8]) -> AmpMsg<'_>` — extracts `version = buf[0] >> 4`, `argc = buf[0] & 0xf`; cursor starts at `buf + 1`.
- `impl AmpMsg { pub fn amp_decode_arg(&mut self) -> Vec<u8> }` — reads a 4-byte BE length, copies that many bytes into a fresh `Vec<u8>` (mirroring the C `malloc` + `memcpy`), and advances the cursor.

### Deviations from the C semantics

- Ownership: the C API returns `char *` (caller frees); the Rust API returns owned `Vec<u8>` / `Vec<u8>` per arg, so there is no manual `free`.
- `argc` is still masked to 4 bits (`& 0xf`) and the header packs `version << 4 | argc`, exactly as in C.
- No `Result`/error types: out-of-range cursor access would panic, matching the "no error handling" C behavior; the happy path used by the tests never panics.
- Added an empty `[workspace]` table to `Cargo.toml` because the workspace directory sits inside a parent cargo workspace on disk; this keeps the package standalone without touching the parent manifest.

### cargo test result

```
     Running unittests src/lib.rs (target/debug/deps/amp-843f5233617e62f6)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/test.rs (target/debug/deps/test-4484db73d39b5139)

running 1 test
test test_roundtrip ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests amp

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Verification (ledger worker)

- Listed and displayed Cargo.toml, src/lib.rs, tests/test.rs, notes.md (contents unchanged).
- Ran `cargo clean && cargo test`: fresh compile of `amp v0.1.0`, exit code 0.
  - unittests src/lib.rs: 0 tests, ok
  - tests/test.rs: `test_roundtrip ... ok` — 1 passed; 0 failed
  - Doc-tests amp: 0 tests, ok
- Ran `cargo build`: `Finished dev profile`, exit code 0, no errors or warnings.
- No code was modified.
