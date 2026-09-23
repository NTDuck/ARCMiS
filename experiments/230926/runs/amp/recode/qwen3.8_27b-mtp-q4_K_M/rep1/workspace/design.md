# AMP → Rust Translation Design

## 1. Source project research

**Project:** `amp` — a C implementation of the "Abstract Message Protocol"
(https://github.com/visionmedia/node-amp), a tiny wire format for encoding an
argv-style message: one header byte (`version << 4 | argc`) followed by, for
each argument, a big-endian u32 length and the raw bytes.

### File inventory

| File | Role |
|---|---|
| `src/amp.h` | Public API: `AMP_VERSION` (1), `amp_t` struct (`short version; short argc; char *buf;`), prototypes for `amp_encode`, `amp_decode`, `amp_decode_arg`. |
| `src/amp.c` | Implementation: `read_u32_be` / `write_u32_be` (static helpers), `amp_decode` (parses header, advances cursor), `amp_decode_arg` (reads u32be length, `malloc`s + `memcpy`s the arg, advances cursor), `amp_encode` (computes total length, `malloc`s, writes header + per-arg length/data). |
| `tests/test.c` | Single test: encodes `{"some","stuff","here"}`, decodes header (asserts version==1, argc==3), decodes the 3 args and string-compares them. Prints `ok`. |
| `Makefile` | gcc build with coverage flags; `make` builds `test.out` and runs it; `make test` runs it; `make clean` removes artifacts. |
| `package.json` | clibs metadata only (`src: ["amp.c","amp.h"]`); no npm dependencies. |
| `Readme.md` | Usage example + performance note (~10m ops/s in C). |

### Public interface (C)

```c
#define AMP_VERSION 1

typedef struct { short version; short argc; char *buf; } amp_t;

char *amp_encode(char **argv, int argc);   // caller frees
void  amp_decode(amp_t *msg, char *buf);   // msg.buf becomes cursor
char *amp_decode_arg(amp_t *msg);          // caller frees; NULL on OOM
```

### Protocol details (must be preserved byte-for-byte)

- Header byte: `version` in high nibble, `argc` in low nibble → **argc is
  limited to 0..=15** by the protocol.
- Each argument: 4-byte big-endian length, then that many raw bytes.
- No NUL terminators, no framing beyond the header; the buffer is fully
  described by the lengths.
- Endianness: big-endian (network order) for the u32 lengths.

### Build/test setup

- C: `make` → compile `tests/test.c` + `src/amp.c` → run binary; success =
  exit 0 + `ok` printed.
- Target: **`cargo test`** (per task).

## 2. Third-party library analysis

The C project has **zero third-party dependencies** (only libc: `string.h`,
`stdlib.h`, `stdint.h`). Therefore the Rust translation needs **no external
crates** — everything is expressible with `std`:

| C dependency | Rust counterpart | Notes |
|---|---|---|
| `string.h` (`strlen`, `memcpy`) | `std::str::len`, `slice::copy_from_slice` / `Vec` | Rust slices are length-based; no `strlen` needed. |
| `stdlib.h` (`malloc`/`free`) | Rust ownership (`Vec<u8>`, `String`) | No manual free; `decode_arg` returns owned or borrowed data instead of a `malloc`'d pointer. |
| `stdint.h` (`uint32_t`) | `u32` | Use `u32::from_be_bytes` / `to_be_bytes` instead of manual shifts (avoids the C signed-`char` shift UB). |
| `assert.h` (tests) | `assert!` / `assert_eq!` in `#[cfg(test)]` or `tests/` | Same semantics. |

No version pinning required; `edition = "2021"` (or 2024) with no
`[dependencies]` entries.

## 3. Target project design

### Crate layout

```
amp/
├── Cargo.toml          # name = "amp", edition 2021, no deps
├── src/
│   └── lib.rs          # public API + unit tests (mirrors src/amp.c + amp.h)
└── tests/
    └── test.rs         # integration test mirroring tests/test.c
```

### Public API (Rust)

```rust
/// Protocol version (high nibble of the header byte).
pub const VERSION: u8 = 1;

/// A decoded AMP message with a cursor over the remaining argument bytes.
pub struct Message<'a> {
    pub version: u8,
    pub argc: u8,
    cursor: &'a [u8],
}

impl<'a> Message<'a> {
    /// Decode the header from `buf` (must be non-empty).
    /// Equivalent of `amp_decode`.
    pub fn decode(buf: &'a [u8]) -> Self;

    /// Decode the next argument, advancing the cursor.
    /// Equivalent of `amp_decode_arg`; returns None if the buffer is
    /// exhausted or the declared length overflows the remaining bytes.
    pub fn decode_arg(&mut self) -> Option<&'a [u8]>;
}

/// Encode an argv into an AMP message buffer.
/// Equivalent of `amp_encode`. Returns Err if argc > 15 (protocol limit).
pub fn encode(argv: &[&str]) -> Result<Vec<u8>, AmpError>;
```

Design decisions:

1. **Borrowed decode, owned encode.** C's `amp_decode`/`amp_decode_arg` hand
   out `malloc`'d buffers the caller must free. In Rust, `Message` borrows
   the input buffer (`'a`) and `decode_arg` returns `Option<&'a [u8]>` —
   zero-copy, no leaks possible. `encode` returns an owned `Vec<u8>`.
2. **`decode_arg` returns `Option`** rather than panicking: mirrors C's
   `NULL` return and is safe against truncated input. (C's OOM-NULL becomes
   unreachable; truncation is the real failure mode.)
3. **`encode` returns `Result`** for the argc>15 case, which C silently
   corrupts (argc is masked to 4 bits). Alternatively we could mask like C;
   the design chooses to surface the error, but the happy path (argc ≤ 15)
   is byte-identical to C.
4. **Big-endian u32** via `u32::from_be_bytes`/`to_be_bytes` — identical
   wire bytes to `read_u32_be`/`write_u32_be`, without C's signed-char
   shift undefined behavior.
5. **No VLA**: C's `size_t lens[argc]` (variable-length array, a GCC
   extension) becomes a `Vec<u8>` length computation or a two-pass
   `Vec::with_capacity` build.
6. **Tests**:
   - `tests/test.rs` mirrors `tests/test.c` exactly: encode
     `["some","stuff","here"]`, assert `version == 1`, `argc == 3`, decode
     the three args and compare to `b"some"`, `b"stuff"`, `b"here"`.
   - Optional extra unit tests in `lib.rs`: round-trip with empty argv,
     empty-string argument, single argument, and a truncated-buffer
     `decode_arg` → `None` case.

### Cargo.toml

```toml
[package]
name = "amp"
version = "0.0.1"
edition = "2021"
description = "Abstract Message Protocol"
license = "MIT"

[dependencies]
```

### Test command

`cargo test` — runs both the `tests/test.rs` integration test and any
`#[cfg(test)]` unit tests.

## 4. Translation risks

1. **argc > 15**: C masks argc into 4 bits (silent corruption); the Rust
   design returns `Err`. Behavior diverges only on invalid input — acceptable
   and safer, but noted.
2. **Ownership model change**: C callers `free()` the results of
   `amp_encode`/`amp_decode_arg`; Rust callers get slices/Vecs. Any
   downstream consumer code must be rewritten, but this project is
   self-contained (only the test consumes the API).
3. **`amp_t.buf` is a mutable cursor in C** (`char *buf` advanced in place).
   Rust models this as a private `cursor: &'a [u8]` field advanced via
   `&mut self` — same semantics, but the field is no longer publicly
   writable, which is stricter (good).
4. **Signedness/UB in C helpers**: `buf[0] << 24` on a possibly-negative
   `char` is UB in C; Rust's `u8`-based `from_be_bytes` is well-defined.
   Wire output is identical for valid inputs.
5. **Null/empty edge cases**: C's `amp_encode` with `argc == 0` writes only
   the header byte; Rust must preserve that (encode of `&[]` → single byte
   `0x10`). Empty-string args (length 0) must round-trip.
6. **No external deps** means no version-pinning risk; the only risk is
   edition/API stability, which is minimal for `std` primitives used.
