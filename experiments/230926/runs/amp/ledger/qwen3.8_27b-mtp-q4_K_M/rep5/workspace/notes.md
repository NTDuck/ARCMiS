# Ledger worker notes — amp (C → Rust)

## Source inventory (sources/)
- `src/amp.h` — `AMP_VERSION 1`; `amp_t { short version; short argc; char *buf; }`;
  prototypes `amp_encode(char **argv, int argc) -> char*`,
  `amp_decode(amp_t*, char*)`, `amp_decode_arg(amp_t*) -> char*`.
- `src/amp.c` — big-endian u32 helpers (`read_u32_be`/`write_u32_be`),
  header decode (byte 0: `version = b0 >> 4`, `argc = b0 & 0xf`, cursor = buf+1),
  `amp_decode_arg` (reads u32be len, mallocs a copy, advances cursor),
  `amp_encode` (1 header byte + per-arg [u32be len][data]).
- `tests/test.c` — encodes `{"some","stuff","here"}`, decodes header
  (asserts version==1, argc==3), decodes 3 args (asserts "some","stuff","here"),
  prints "ok".
- `Makefile` — builds `test.out` and runs it; `package.json` — clibs metadata
  (name "amp", version 0.0.1, MIT); `Readme.md` — protocol example.

## Brainstorm

### Core difficulties
1. **`amp_t` mixes data and a cursor into a caller-owned buffer.** In C,
   `msg.buf` points *into* the caller's `buf`; `amp_decode_arg` advances it and
   returns a malloc'd copy. Rust options:
   - (a) `struct Amp { version: u8, argc: u8, buf: Vec<u8>, pos: usize }` —
     decode copies the payload into the struct; decode_arg slices `buf[pos..]`
     and advances `pos`. Simple, owns its memory, no lifetimes. **Chosen.**
   - (b) `struct Amp<'a> { version: u8, argc: u8, buf: &'a [u8], pos: usize }`
     — zero-copy, but borrows complicate the API and the C test style
     (decode into a struct, then repeatedly decode args) still works; rejected
     for simplicity since the C code copies args anyway.
   - (c) Iterator/`Read`-trait style — overkill for a 3-function API.
2. **Memory management.** C: `malloc`/caller-`free` for `amp_encode` result and
   each decoded arg. Rust: return `Vec<u8>` from both; ownership transfer
   replaces the free contract. No `Box`/raw pointers needed.
3. **`char*` vs bytes.** Args are byte strings; use `&[u8]`/`&str` for input
   and `Vec<u8>` for output. `amp_encode(&[&str]) -> Vec<u8>` reads most
   naturally and matches the C `char **argv` shape.
4. **Big-endian u32.** Use `u32::from_be_bytes` / `u32::to_be_bytes` instead of
   the manual shift code.
5. **Error handling.** C returns `NULL` on OOM; Rust allocation panics on OOM —
   no `Option`/`Result` needed to stay faithful. Bounds: C reads past the end
   silently; Rust will panic on out-of-range slices, which is acceptable (tests
   never do that) and arguably better.
6. **Types.** `short version/argc` → `u8` (both fit in 4 bits per the protocol;
   `u8` is the honest width). Keep `AMP_VERSION` as `pub const AMP_VERSION: u8 = 1`.
7. **Crate layout.** `Cargo.toml` (name `amp`, version `0.0.1`, edition 2021,
   license MIT), `src/lib.rs` with the public API, integration test
   `tests/test.rs` mirroring `tests/test.c` (keeps the tests/ directory
   parallel to the C project). `cargo test` replaces the Makefile's
   build-and-run step.
8. **API shape.** C mutates `amp_t` in place; Rust: `amp_decode(&mut Amp, &[u8])`
   and `amp_decode_arg(&mut Amp) -> Vec<u8>` mirror the C signatures 1:1, making
   the test translation mechanical.

### Candidate approaches (summary)
- **A (chosen):** owned `Amp` struct with `Vec<u8>` + `pos` cursor;
  `Vec<u8>` returns; `u32::from_be_bytes`; `src/lib.rs` + `tests/test.rs`.
- **B:** lifetime-based zero-copy `Amp<'a>` — more "Rust-y" but more friction,
  no benefit for this size.
- **C:** iterator-based decoder — cleaner for streaming but diverges from the
  C API the tests are written against.

### Test parity checklist
- `amp_encode(["some","stuff","here"])` → non-empty `Vec<u8>`.
- `amp_decode` → `version == 1`, `argc == 3`.
- Three `amp_decode_arg` calls → `b"some"`, `b"stuff"`, `b"here"`.
- (Optional extra) round-trip byte-exactness of the encoded buffer.

## Ledger worker — Rust translation of amp (final)
- Verified existing crate: Cargo.toml (name `amp`, edition 2021), src/lib.rs
  (AMP_VERSION, Amp struct, amp_encode/amp_decode/amp_decode_arg), tests/test.rs
  (ported tests/test.c + byte-exact round-trip test).
- Fixed one failure: the wire-format diagram in `amp_encode`'s doc comment was
  parsed by rustdoc as a Markdown indented code block and run as a doctest,
  failing compilation. Wrapped the diagram in a ```text fence so it is
  documentation only.
- `cargo test` now fully green: 2 integration tests pass, 0 doctests, no warnings.

## Verification round (no code changes)
- Listed and displayed Cargo.toml, src/lib.rs, tests/test.rs.
- Ran `cargo clean` then `cargo test` from scratch: 2/2 integration tests passed (test, test_byte_exact), 0 unit/doc tests, no failures.
- Cross-checked Rust API vs sources/src/amp.c + amp.h + tests/test.c:
  - Header byte: encode `AMP_VERSION << 4 | argc`, decode `buf[0] >> 4` / `buf[0] & 0xf` — identical in Rust.
  - Lengths: C read_u32_be/write_u32_be == Rust to_be_bytes/from_be_bytes (big-endian u32).
  - Cursor: C advances msg->buf by 4 then len; Rust advances msg.pos by 4 then len. Equivalent.
  - Test parity: both assert version==1, argc==3, args "some"/"stuff"/"here". Rust additionally has test_byte_exact (extra, not a discrepancy).
- Discrepancies: none affecting semantics. Only idiomatic differences (Rust Vec<u8> return vs C malloc'd char*, u8 vs short fields, Rust amp_decode copies payload vs C aliasing caller buffer, Rust panics on malformed input vs C UB).
