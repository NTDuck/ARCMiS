# AMP → Rust Translation Design

## 1. Source Project Research

### Overview
AMP is a tiny C implementation of the "Abstract Message Protocol" (originally
`node-amp` by TJ Holowaychuk). It encodes an argv-style message into a binary
buffer and decodes it back. MIT licensed, distributed as a clibs package
(`package.json`, repo `clibs/amp`, version 0.0.1).

### File inventory
| File | Role |
|---|---|
| `src/amp.h` | Public API: `AMP_VERSION` (1), `amp_t` struct, 3 function prototypes |
| `src/amp.c` | Implementation: u32be read/write, `amp_encode`, `amp_decode`, `amp_decode_arg` |
| `tests/test.c` | Single test: encode 3 args, decode header, decode each arg, assert equality |
| `Makefile` | gcc build with `-fprofile-arcs -ftest-coverage`; `make` builds `test.out` and runs it |
| `package.json` | clibs metadata (name `amp`, src `amp.c`/`amp.h`) |
| `Readme.md` | Usage example + protocol description |

### Public API (C)
```c
#define AMP_VERSION 1

typedef struct { short version; short argc; char *buf; } amp_t;

char *amp_encode(char **argv, int argc);   // malloc'd buffer, caller frees
void  amp_decode(amp_t *msg, char *buf);   // parses 1-byte header, advances msg->buf
char *amp_decode_arg(amp_t *msg);          // malloc'd copy of next arg, advances cursor
```

### Wire format
```
------------+----------+------------+ ...
| <ver/argc> | <length> | <data>     | additional arguments
------------+----------+------------+
```
- Byte 0: version in high nibble (`buf[0] >> 4`), argc in low nibble (`buf[0] & 0xf`).
- Each argument: 4-byte **big-endian** length, then that many raw bytes.
- Arguments are length-prefixed, **not** NUL-terminated — they may contain arbitrary bytes.
- argc is limited to 15 by the 4-bit field (the C code does not validate this).

### Build/test setup
- `make` → compiles `tests/test.c` + `src/amp.c` into `test.out`, runs it; prints `ok` on success.
- No external dependencies; libc only (`string.h`, `stdlib.h`, `stdint.h`).

## 2. Third-Party Library Analysis

The C project has **zero third-party dependencies** (libc only). Therefore the
Rust port needs **no external crates** — everything is expressible with `std`:

| C dependency | Rust counterpart | Notes |
|---|---|---|
| `string.h` (`strlen`, `memcpy`) | `std::slice` / `Vec::extend_from_slice` | Lengths come from the protocol, not NUL termination |
| `stdlib.h` (`malloc`) | `Vec<u8>` / `Box<[u8]>` | Ownership replaces manual free |
| `stdint.h` (`uint32_t`) | `u32` | Use `u32::to_be_bytes` / `u32::from_be_bytes` instead of manual shifts |
| `assert.h` (tests) | `assert!` / `assert_eq!` | Built into std |

## 3. Target Project Design (Rust)

### Layout
```
amp/
├── Cargo.toml
├── src/
│   └── lib.rs          # the whole library (small enough for one file)
└── tests/
    └── test.rs         # integration test mirroring tests/test.c
```

### Cargo.toml
```toml
[package]
name = "amp"
version = "0.0.1"
edition = "2021"
description = "Abstract Message Protocol"
license = "MIT"
keywords = ["amp", "tcp", "udp", "message", "protocol", "encode", "decode"]

[dependencies]
```

### API design (idiomatic Rust)
```rust
pub const VERSION: u8 = 1;

/// Decoded AMP message header with a cursor into the payload.
pub struct AmpMessage {
    pub version: u8,
    pub argc: u8,
    buf: &[u8],   // remaining payload; advanced by decode_arg
}

/// Encode an argv into an AMP message buffer.
pub fn encode(argv: &[&[u8]]) -> Vec<u8>;

/// Parse the 1-byte header of `buf` into `msg`.
/// Returns Err if the buffer is shorter than 1 byte.
pub fn decode(msg: &mut AmpMessage, buf: &[u8]) -> Result<(), AmpError>;

/// Decode the next argument, advancing the cursor.
/// Returns Err if the buffer is truncated (bad length or missing data).
pub fn decode_arg(msg: &mut AmpMessage) -> Result<Vec<u8>, AmpError>;
```

Design decisions:
- **Bytes, not strings**: arguments are length-prefixed binary, so the API
  takes/returns `&[u8]` / `Vec<u8>`. A convenience `encode_str(argv: &[&str])`
  can be added, but the core stays byte-oriented (matches C semantics exactly).
- **Borrowed cursor**: `AmpMessage` holds a `&[u8]` slice (not an owned copy),
  so `decode_arg` can return `Result<&[u8], _>` borrowing from the message —
  no per-argument allocation, unlike the C `malloc` per arg. We can expose
  `decode_arg` returning `Result<&[u8], AmpError>` (zero-copy) — this is the
  idiomatic improvement over C.
- **Errors**: C signals failure with `NULL`; Rust uses a small
  `AmpError` (e.g. `Truncated`, `InvalidArgc`) or simply `std::io::Error`-style
  enum. `encode` can't fail (no fallible allocation in safe Rust), so it
  returns `Vec<u8>` directly.
- **Validation**: `encode` should reject `argc > 15` (the 4-bit field) — the
  C code silently corrupts the header for argc ≥ 16; returning an error or
  panicking is safer. `decode` should validate `buf.len() >= 1`.
- **Big-endian I/O**: use `u32::to_be_bytes` / `u32::from_be_bytes` instead of
  the manual shift code in `amp.c`.
- **Types**: C `short` → `u8` (both fields fit in 4 bits anyway).

### Tests
`tests/test.rs` mirrors `tests/test.c`:
```rust
use amp::{encode, decode, decode_arg, AmpMessage, VERSION};

#[test]
fn roundtrip() {
    let args: Vec<&[u8]> = ["some", "stuff", "here"].iter().map(|s| s.as_bytes()).collect();
    let buf = encode(&args);
    let mut msg = AmpMessage::default();
    decode(&mut msg, &buf).unwrap();
    assert_eq!(msg.version, VERSION);
    assert_eq!(msg.argc, 3);
    for expected in ["some", "stuff", "here"] {
        let arg = decode_arg(&mut msg).unwrap();
        assert_eq!(arg, expected.as_bytes());
    }
}
```
Plus unit tests in `lib.rs` (`#[cfg(test)]`): empty argv, single arg,
binary argument containing NUL bytes, truncated-buffer errors, argc > 15
rejection.

Test command: `cargo test` (runs both unit and integration tests).

## 4. Translation Risks

1. **argc overflow**: C packs argc into 4 bits without checking; a Rust port
   must decide the behavior for `argc > 15` (error vs. panic). Choose `Result`
   or `debug_assert` + documented limit.
2. **NUL-termination assumption**: naive translation might use `str`/`CString`;
   the protocol is length-prefixed and allows NUL bytes — must stay byte-based.
3. **Ownership model**: C returns `malloc`'d buffers the caller must free;
   Rust's zero-copy slice-based `decode_arg` changes the API shape (borrow
   lifetime tied to the message). This is an improvement but a semantic
   difference to document.
4. **Error propagation**: C uses `NULL` returns; Rust must thread `Result`s
   through `decode`/`decode_arg`, which changes the call-site ergonomics.
5. **Endianness**: must remain big-endian on the wire regardless of host
   architecture — `to_be_bytes`/`from_be_bytes` handle this correctly.
6. **Length type**: C uses `uint32_t` for lengths; keep `u32` (not `usize`)
   for wire fidelity, converting to slice indices carefully (lengths > buffer
   size must be rejected, not wrapped).
7. **Coverage flags**: the Makefile's gcov instrumentation has no direct
   `cargo test` equivalent; not required for the port (use `cargo-llvm-cov`
   optionally, but no dependency is added).
