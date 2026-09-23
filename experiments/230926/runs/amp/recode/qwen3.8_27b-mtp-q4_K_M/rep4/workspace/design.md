# AMP — C → Rust Translation Design

## 1. Source Project Research

### Overview
`amp` is a tiny C library (clibs package, MIT license) implementing the
"Abstract Message Protocol" (AMP) — a wire format for encoding an argv-style
message as:

```
------------+------------+------------+  (repeated per argument)
| <ver/argc> | <length>   | <data>     |
| 1 byte     | 4 bytes BE | <len> bytes|
------------+------------+------------+
```

- Byte 0: high nibble = protocol version (1), low nibble = argument count (0–15).
- Each argument: 4-byte big-endian length followed by raw bytes.

### File inventory
| File | Role |
|---|---|
| `src/amp.h` | Public API: `AMP_VERSION` (1), `amp_t` struct (`short version; short argc; char *buf;`), prototypes for `amp_encode`, `amp_decode`, `amp_decode_arg` |
| `src/amp.c` | Implementation: `read_u32_be` / `write_u32_be` (static), header decode, arg decode (malloc + memcpy, advances cursor), encode (single malloc, writes header + args) |
| `tests/test.c` | Round-trip test: encode `["some","stuff","here"]`, decode header (assert version==1, argc==3), decode 3 args and compare strings |
| `Makefile` | gcc build with coverage flags; `make` builds `test.out` and runs it; `make clean` |
| `package.json` | clibs metadata only (name, src list) — not a real Node dependency manifest |
| `Readme.md` | Usage example + performance note (~10m ops/s in C) |

### Public interface (C)
```c
#define AMP_VERSION 1
typedef struct { short version; short argc; char *buf; } amp_t;

char *amp_encode(char **argv, int argc);   // malloc'd buffer, NULL on OOM
void  amp_decode(amp_t *msg, char *buf);   // fills version/argc, sets cursor
char *amp_decode_arg(amp_t *msg);          // malloc'd copy, advances cursor, NULL on OOM
```

### Behavioral notes / C quirks
- `argc` is limited to 15 (4-bit field); the C code does not validate this.
- `amp_decode_arg` does **no bounds checking** — decoding a truncated buffer is UB in C.
- `amp_encode` does not validate `argc <= 15` either.
- Decoded args are heap copies the caller must free; the message buffer itself is caller-owned.
- No third-party dependencies at all (libc only: `string.h`, `stdlib.h`, `stdint.h`).

## 2. Third-Party Library Analysis

The C project has **zero third-party dependencies** — only libc. Therefore the
Rust translation needs **no external crates**; everything is expressible with
`std` (slices, `Vec<u8>`, `u32::to_be_bytes`/`from_be_bytes`).

| C dependency | Rust counterpart |
|---|---|
| `string.h` (`strlen`, `memcpy`) | `std::slice` / `Vec` (no crate) |
| `stdlib.h` (`malloc`) | `Vec<u8>` / `Box` (allocator built in) |
| `stdint.h` (`uint32_t`) | `u32` primitive |
| `assert.h` (tests) | `assert!` / `#[test]` (built in) |

No version/API differences to track.

## 3. Target Project Design (Rust)

### Cargo layout
```
amp/
├── Cargo.toml          # [package] name = "amp", edition = "2021", no deps
├── src/
│   └── lib.rs          # the whole library (small enough for one file)
└── tests/
    └── test.rs         # port of tests/test.c
```

`Cargo.toml`:
```toml
[package]
name = "amp"
version = "0.0.1"
edition = "2021"
description = "Abstract Message Protocol"
license = "MIT"

[dependencies]
```

### API design (idiomatic Rust)

Replace raw pointers and manual memory management with slices and owned
`Vec<u8>`. The cursor (`amp_t.buf`) becomes a `&'a [u8]` slice that shrinks as
args are decoded — zero-copy, no malloc per arg.

```rust
/// Protocol version (high nibble of byte 0).
pub const VERSION: u8 = 1;

/// A decoded AMP message with a cursor into the remaining payload.
pub struct AmpMessage<'a> {
    pub version: u8,
    pub argc: u8,
    buf: &'a [u8],   // cursor: remaining undecoded payload
}

impl<'a> AmpMessage<'a> {
    /// Decode the header from `buf` (must be non-empty).
    /// Equivalent of `amp_decode`.
    pub fn new(buf: &'a [u8]) -> Result<Self, AmpError>;

    /// Decode the next argument as a slice borrowed from the message buffer.
    /// Equivalent of `amp_decode_arg` (but zero-copy, no free needed).
    pub fn decode_arg(&mut self) -> Result<&'a [u8], AmpError>;
}

/// Encode an argv into an AMP message buffer.
/// Equivalent of `amp_encode`.
pub fn encode(argv: &[&[u8]]) -> Result<Vec<u8>, AmpError>;

#[derive(Debug, PartialEq)]
pub enum AmpError {
    EmptyBuffer,        // decode called on empty input
    TooManyArgs,        // argc > 15 (4-bit field)
    TruncatedArgument,  // not enough bytes for length or data
    ArgMismatch,        // more decode_arg calls than argc (optional strictness)
}
```

Design decisions:
- **`encode` takes `&[&[u8]]`** — generic over `&str`, `&[u8]`, `String` via
  `AsRef<[u8]>` if desired; simplest faithful port is `&[&[u8]]`.
- **`decode_arg` returns `Result<&'a [u8], AmpError>`** instead of a malloc'd
  copy: the caller borrows from the original buffer (which outlives the
  message), so no allocation and no `free`. This is the idiomatic Rust
  improvement over the C API.
- **Errors instead of NULL/UB**: C returns NULL only on OOM and reads out of
  bounds on truncated input; Rust bounds-checks and returns `AmpError`.
- **`argc` validation**: `encode` returns `Err(TooManyArgs)` if `argv.len() > 15`
  (C silently corrupts the header).
- **Endianness**: use `u32::to_be_bytes()` / `u32::from_be_bytes()` instead of
  manual shift/mask helpers.
- **Version/argc types**: `u8` (C used `short`; values fit in a byte).

### Test port (`tests/test.rs`)
```rust
use amp::{encode, AmpMessage};

#[test]
fn round_trip() {
    let args: Vec<&[u8]> = vec![b"some", b"stuff", b"here"];
    let buf = encode(&args).unwrap();

    let mut msg = AmpMessage::new(&buf).unwrap();
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 3);

    assert_eq!(msg.decode_arg().unwrap(), b"some");
    assert_eq!(msg.decode_arg().unwrap(), b"stuff");
    assert_eq!(msg.decode_arg().unwrap(), b"here");
}
```
Plus extra tests the C version lacks (worth adding):
- truncated buffer → `TruncatedArgument`
- `encode` with 16 args → `TooManyArgs`
- empty argv (argc = 0) round-trip
- argument containing arbitrary bytes (e.g. `\0` inside) — the length-prefixed
  format supports it; C `strcmp`-based tests couldn't.

### Build/test
- `cargo build` — compiles the lib.
- `cargo test` — runs `tests/test.rs` (matches the required test command).
- No Makefile needed; `cargo clean` replaces `make clean`.

## 4. Translation Risks

1. **API shape change (NULL → Result, malloc → borrow)**: any downstream C
   consumer expecting `char*` + `free` semantics gets a different contract.
   Within this repo (only `tests/test.c` consumes the API) this is safe and is
   the idiomatic choice.
2. **`argc` > 15**: C silently truncates the count into 4 bits; Rust will
   reject it. Behavior change, but strictly safer; documented in the error type.
3. **Truncated/malformed input**: C is UB; Rust returns `Err`. Tests must not
   assume success on bad input.
4. **Lifetime coupling**: `decode_arg` borrows from the caller's buffer; the
   buffer must outlive all decoded slices. This is enforced at compile time,
   but API users must keep the original `Vec<u8>` alive (unlike C where each
   arg was an independent copy).
5. **`version`/`argc` field types**: C `short` → Rust `u8`; no practical risk
   (values are 0–15 / 1), but a faithful "struct field" mapping would be `u16`
   if strictness is preferred. `u8` chosen for idiomaticity.
6. **No dependencies to pin** — no supply-chain or version-drift risk.
