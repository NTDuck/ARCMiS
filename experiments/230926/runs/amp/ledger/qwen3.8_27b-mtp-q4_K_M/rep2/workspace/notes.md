# Notes

## 2025-09-23 — Planning phase (ledger worker)

- Task: plan the C -> Rust translation of the `amp` protocol library.
- **Caveat:** the C sources listed in the task (Makefile, src/amp.c,
  src/amp.h, tests/test.c) were NOT present in the workspace (it was empty).
  The plan is based on the API surface given in the task message and the
  well-known amp wire format; re-verify against the C files if they appear.
- Wrote `plan.md` (strategy, core difficulties, candidate approaches,
  recommended idiomatic-Rust API) and `tasks.json` (4 seed tasks:
  scaffold, implement, port-tests, verify).
- Recommended approach: idiomatic Rust — `Vec<u8>` encode output, `&[u8]`
  input slices, offset-cursor `AmpDecoder` for `amp_decode_arg`, `Result`
  with an `AmpError` enum instead of C's -1/NULL.
- No implementation code written (brainstorm phase only).

## 2025-09-23 — Implementation phase (ledger worker)

- Translated the REAL C source provided in the task (src/amp.h, src/amp.c,
  tests/test.c). NOTE: the planning phase assumed a 4-byte header
  (version<<28 | argc) and a Result-based API; the real C uses a 1-byte
  header `(version << 4) | argc` and plain pointers. The implementation
  follows the real C, per the task instruction.
- Files created:
  - `Cargo.toml` — package "amp", edition 2021, no dependencies. Added an
    empty `[workspace]` table because an unrelated cargo workspace exists
    outside the workspace root and was claiming this package.
  - `src/lib.rs` — API:
    - `pub const AMP_VERSION: u8 = 1;`
    - `pub struct AmpMessage { pub version: u16, pub argc: u16, buf: Vec<u8>, pos: usize }`
      (models C `amp_t`; `buf`+`pos` model the `char *buf` cursor)
    - `pub fn amp_encode(argv: &[&str]) -> Vec<u8>` — header byte then per-arg
      4-byte BE length + raw bytes (byte-identical to C amp_encode).
    - `pub fn amp_decode(buf: &[u8]) -> AmpMessage` — version = byte0 >> 4,
      argc = byte0 & 0xf, cursor after byte 0.
    - `pub fn amp_decode_arg(msg: &mut AmpMessage) -> Vec<u8>` — reads BE u32
      length, copies the bytes (C malloc+memcpy), advances cursor.
    - No unsafe, no raw pointers, no dependencies.
  - `tests/amp.rs` — `#[test] encode_decode_round_trip` mirroring tests/test.c:
    encode ["some","stuff","here"], assert version==1, argc==3, decode the
    three args and assert "some"/"stuff"/"here".
- Results:
  - `cargo build`: Finished `dev` profile, exit 0.
  - `cargo test`: 1 passed; 0 failed (encode_decode_round_trip ok; 0 unit
    tests, 0 doc-tests), exit 0.
- All four tasks in tasks.json marked "done".

## Verification pass (independent)
- Confirmed Cargo.toml, src/lib.rs, tests/amp.rs present.
- `cargo build`: exit 0. `cargo test`: exit 0, 1 passed / 0 failed (integration test `encode_decode_round_trip`).
- Independent wire-format check (temporary test, removed after): amp_encode(["ab","c"]) == [0x12, 00 00 00 02, 'a','b', 00 00 00 01, 'c']; hand-built decode of [0x11, 00 00 00 03, 00 01 02] yields version=1, argc=1, arg=[0,1,2]. Byte-exact match with C semantics.
- Verdict: PASS.
