# Plan: C -> Rust translation of the `amp` protocol library

> Note: the C sources (Makefile, src/amp.c, src/amp.h, tests/test.c) were not
> present in the workspace at planning time. This plan is based on the API
> surface stated in the task (amp_encode, amp_decode, amp_decode_arg, amp_t,
> AMP_VERSION) and the well-known wire format of the amp protocol. If the C
> files appear before implementation, re-verify the details against them.

## Wire format (protocol)

- 4-byte header, big-endian u32: `(AMP_VERSION << 28) | argc`
  (version in the high 4 bits, argc in the low 28 bits).
- Then, per argument: 4-byte big-endian u32 length, followed by that many
  raw bytes.
- `AMP_VERSION` = 1.

## Core difficulties of the translation

1. **Ownership / malloc semantics.** C `amp_encode` returns a `malloc`'d
   `unsigned char *` whose length is written through an out-parameter
   (`size_t *len`); `amp_decode`/`amp_decode_arg` hand back a `malloc`'d
   `char **` array of `malloc`'d strings that the caller must `free`.
   Rust has no manual free: encode should return an owned `Vec<u8>` (length
   is implicit), decode should return `Vec<String>`.
2. **Mutable cursor in `amp_t`.** C `amp_t { buf, len, pos }` is advanced in
   place by `amp_decode_arg`, and the caller passes `&amp` repeatedly. In
   Rust this maps to a struct holding `&[u8]` + `pos: usize` with a
   `&mut self` method (`next_arg`) — the borrow checker enforces the
   sequential-use pattern the C code relies on.
3. **Byte-level encoding.** u32 big-endian writes, version/argc nibble
   packing (`version << 28 | argc`), and length-prefixed payloads. Straightforward
   with `to_be_bytes()` / `from_be_bytes()`, but the exact bit layout must be
   preserved byte-for-byte (tests assert individual bytes).
4. **Error model.** C returns `-1` / NULL and uses `assert` in tests. Rust
   should use `Result<Vec<String>, AmpError>` / `Result<String, AmpError>`
   with an `AmpError` enum (e.g. `Truncated`, `UnsupportedVersion(u8)`,
   `ArgTooLong`) instead of panicking; the ported test asserts `Err` on the
   EOF case that C asserts as `-1`.
5. **Test porting.** The C test is a `main()` of `assert`s: exact encoded
   bytes for `["foo","bar","baz"]`, a full decode round-trip, and three
   sequential `amp_decode_arg` calls followed by a failure. This becomes a
   single Rust `#[test]` (or a few) using `assert_eq!` / `assert!(matches!(...))`.
6. **Cargo setup.** New crate `amp`, `edition = "2021"`, library target
   (`src/lib.rs`) plus an integration test in `tests/amp.rs`; no external
   dependencies needed.

## Candidate approaches

- **A. Faithful 1:1 port with `unsafe` pointers.** Mirror `amp_t` with raw
  `*mut u8`, return `*mut u8` from encode, use out-params. Rejected: leaks
  Rust's safety guarantees, adds no value, and the C API's out-params are
  exactly what Rust idiom avoids.
- **B. Idiomatic Rust (recommended).** `Vec<u8>` for the encoded buffer,
  `&[u8]` slices for input, an offset-cursor struct for incremental decode,
  `Result` for errors, `Vec<String>` for decoded arguments. Keeps the
  protocol byte-compatible while using Rust ownership and borrowing.

## Recommended approach (B) — target API

```rust
pub const AMP_VERSION: u8 = 1;

pub enum AmpError { Truncated, UnsupportedVersion(u8), ArgTooLong }

/// Equivalent of amp_encode: returns the full encoded buffer.
pub fn amp_encode(argv: &[&str]) -> Vec<u8>;

/// Equivalent of amp_decode: decodes the whole message.
pub fn amp_decode(buf: &[u8]) -> Result<Vec<String>, AmpError>;

/// Equivalent of amp_t + amp_decode_arg: incremental cursor.
pub struct AmpDecoder<'a> { /* buf: &'a [u8], pos: usize */ }
impl<'a> AmpDecoder<'a> {
    pub fn new(buf: &'a [u8]) -> Result<Self, AmpError>; // reads/validates header
    pub fn argc(&self) -> usize;
    pub fn next_arg(&mut self) -> Result<String, AmpError>; // advances pos
}
```

## Strategy

1. Scaffold the Cargo crate (`Cargo.toml`, `src/lib.rs` with the public API
   signatures and `AmpError`).
2. Implement `amp_encode` (header + length-prefixed args), `amp_decode`
   (validate version, iterate args), and `AmpDecoder` (cursor over the same
   per-argument logic, sharing a private `read_arg` helper).
3. Port `tests/test.c` to `tests/amp.rs`: exact-byte assertions for
   `["foo","bar","baz"]`, round-trip decode, three sequential `next_arg`
   calls, then an `Err` on the fourth.
4. `cargo build` and `cargo test`; fix until green.
