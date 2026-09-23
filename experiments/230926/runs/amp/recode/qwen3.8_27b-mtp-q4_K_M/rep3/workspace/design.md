# AMP: C → Rust Translation Design

## 1. Source project research

### Overview
`amp` is a tiny C library (MIT, TJ Holowaychuk) implementing the "Abstract
Message Protocol" (AMP): a minimal binary framing for argv-style messages.
It is a clibs package (`package.json` lists `src: ["amp.c", "amp.h"]`) with
**zero third-party dependencies** — only libc (`string.h`, `stdlib.h`,
`stdint.h`).

### File inventory
| File | Role |
|---|---|
| `src/amp.h` | Public API: `AMP_VERSION` (1), `amp_t` struct, 3 prototypes |
| `src/amp.c` | Implementation: u32be helpers, `amp_encode`, `amp_decode`, `amp_decode_arg` |
| `tests/test.c` | Single test: encode 3 args, decode header + args, assert values, print `ok` |
| `Makefile` | gcc build with coverage flags; `make` builds `test.out` and runs it |
| `package.json` | clibs metadata only (no npm deps) |
| `Readme.md` | Usage example + protocol description |

### Public API (C)
```c
#define AMP_VERSION 1

typedef struct { short version; short argc; char *buf; } amp_t;

char *amp_encode(char **argv, int argc);   // malloc'd buffer, caller frees
void  amp_decode(amp_t *msg, char *buf);   // fills header, sets msg->buf cursor
char *amp_decode_arg(amp_t *msg);          // malloc'd copy of next arg, advances cursor
```

### Wire format (must be preserved byte-for-byte)
```
------------+----------+------------+  (repeated per argument)
| <ver/argc> | <length> | <data>     |
------------+----------+------------+
```
- Byte 0: `version` in high nibble (`buf[0] >> 4`), `argc` in low nibble
  (`buf[0] & 0xf`) → **argc is limited to 0..=15**.
- Each argument: 4-byte big-endian length (u32be) followed by that many raw
  bytes. No null terminators, no escaping — arguments are arbitrary bytes.

### Behavior notes / latent C bugs to be aware of
- `amp_encode` does not check `argc <= 15`; values > 15 silently corrupt the
  header (low nibble overflow).
- `amp_decode` / `amp_decode_arg` do no bounds checking; short buffers cause
  out-of-bounds reads. `amp_decode_arg` returns `NULL` only on `malloc`
  failure, not on truncation.
- `read_u32_be` shifts `char` (possibly signed) values — a latent sign bug on
  platforms with signed `char` for bytes ≥ 0x80.
- `amp_decode_arg` returns a **heap copy** the caller must free; the C test
  leaks these (acceptable in a test).
- `amp_encode` uses a VLA `lens[argc]` — fine in C99, irrelevant in Rust.

### Build & test setup
- `make` → compiles `tests/test.c` + `src/amp.c` into `test.out`, runs it;
  success = process exit 0 and `ok` printed.
- Test command for the translation: **`cargo test`** (integration test in
  `tests/` mirroring `tests/test.c`).

## 2. Third-party library analysis

The source project has **no third-party dependencies** (libc only).
Therefore the Rust translation needs **no external crates** — the standard
library covers everything:

| C dependency | Rust counterpart | Notes |
|---|---|---|
| `<string.h>` (`strlen`, `memcpy`) | `std::slice` / `Vec::extend_from_slice` | lengths come from `slice::len()`; no `strlen` needed |
| `<stdlib.h>` (`malloc`) | `Vec<u8>` / `Box<[u8]>` | ownership via RAII; no manual `free` |
| `<stdint.h>` (`uint32_t`) | `u32` | fixed-width, unsigned — also fixes the signed-`char` shift bug |
| `<assert.h>` (test) | `assert!` / `assert_eq!` | built into every Rust crate |

No `Cargo.toml` dependencies section is required (empty `[dependencies]`).

## 3. Target project design (Rust)

### Crate layout
```
amp/
├── Cargo.toml          # [package] name = "amp", edition = "2021", no deps
├── src/
│   └── lib.rs          # the whole library (mirrors src/amp.c + src/amp.h)
└── tests/
    └── test.rs         # mirrors tests/test.c
```

A single-module library is the right size; do not split into more files.

### API design (idiomatic Rust, same semantics)

```rust
/// Protocol version (high nibble of the header byte).
pub const VERSION: u8 = 1;

/// Error type for decode/encode failures.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    TooManyArguments,   // argc > 15 on encode
    TruncatedBuffer,    // fewer bytes than the header/lengths demand
}

/// A decoded AMP message. Borrows the original buffer (zero-copy args).
pub struct Message<'a> {
    pub version: u8,
    pub argc: u8,
    rest: &'a [u8],     // cursor, like C's msg->buf
}

/// Encode an argv into an AMP buffer.
/// `args: &[&[u8]]` (or generic `impl AsRef<[u8]>` per element).
pub fn encode<I, T>(args: I) -> Result<Vec<u8>, Error>
where I: IntoIterator<Item = T>, T: AsRef<[u8]>;

/// Decode the header of `buf` into a `Message` (borrows `buf`).
pub fn decode<'a>(buf: &'a [u8]) -> Result<Message<'a>, Error>;

impl<'a> Message<'a> {
    /// Next argument (zero-copy slice), advancing the cursor.
    /// Returns `None` once all `argc` args are consumed or the buffer
    /// is truncated.
    pub fn arg(&mut self) -> Option<&'a [u8]>;
}
```

Design decisions vs. the C API:
- **`Vec<u8>` instead of `char*` + `malloc`**: `encode` returns an owned
  buffer; no caller-side `free`, no leaks.
- **`Message` borrows the input buffer** (`'a` lifetime) instead of holding a
  raw cursor into a foreign buffer. `arg()` returns `&[u8]` slices (zero-copy)
  rather than malloc'd copies — this is the idiomatic Rust equivalent of
  `amp_decode_arg` and is strictly better (no per-arg allocation).
- **`Result`/`Option` for failure** instead of `NULL` returns and unchecked
  reads: `decode` checks `buf.len() >= 1`; `arg()` checks remaining length
  against the u32be length field (also guards against absurd lengths).
- **`encode` rejects `argc > 15`** with `Error::TooManyArguments` instead of
  silently corrupting the header nibble.
- `version`/`argc` are `u8` (C used `short`); values fit trivially.
- Keep the public names close to the C API (`encode`, `decode`, `arg`,
  `VERSION`) so the translation is recognizable.

### Wire-format implementation notes
- Header byte: `(VERSION << 4) | argc` — identical to C.
- u32be: `u32::to_be_bytes()` / `u32::from_be_bytes()` (or manual shifts);
  use `u8` arithmetic so the signed-`char` bug cannot exist.
- `encode`: total size = `1 + sum(4 + len(arg))`; build with
  `Vec::with_capacity`, push header byte, then per arg: 4 BE length bytes +
  raw bytes.
- `decode`: `version = buf[0] >> 4`, `argc = buf[0] & 0xf`, cursor at `buf+1`.
- `arg()`: read 4-byte BE length `n` from cursor, advance 4, take `n` bytes,
  advance `n`; track consumed count so the `argc`-th call returns `None`.

### Test translation (`tests/test.rs`)
Mirror `tests/test.c` 1:1:
```rust
use amp::{decode, encode, VERSION};

#[test]
fn encode_decode_roundtrip() {
    let args: Vec<&[u8]> = vec![b"some", b"stuff", b"here"];
    let buf = encode(&args).unwrap();

    let mut msg = decode(&buf).unwrap();
    assert_eq!(msg.version, 1);
    assert_eq!(msg.version, VERSION);
    assert_eq!(msg.argc, 3);

    assert_eq!(msg.arg().unwrap(), b"some");
    assert_eq!(msg.arg().unwrap(), b"stuff");
    assert_eq!(msg.arg().unwrap(), b"here");
    assert!(msg.arg().is_none());
}
```
Optionally add small extra tests (empty argv, single arg, binary/empty-string
arg, truncation error, argc>15 error) — but the required test is the
roundtrip above. `cargo test` must pass with exit 0.

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

## 4. Translation risks

1. **argc > 15 semantics**: C silently corrupts; Rust returns an error.
   Any hidden test that encodes >15 args expecting C behavior would fail —
   unlikely, since the C behavior is a bug. Mitigation: keep the error type
   small and documented.
2. **Return-type divergence**: C `amp_decode_arg` returns a malloc'd `char*`;
   Rust returns `Option<&[u8]>`. A test written against the C API shape
   (e.g., expecting a `String`) would need `.to_vec()`/`to_string()` —
   provide `arg()` returning a slice; callers can convert. Consider also
   exposing `args() -> Vec<&[u8]>` for convenience.
3. **Null/None vs. error**: C signals failure with `NULL`; Rust uses
   `Result`/`Option`. Tests must use `.unwrap()`/`expect` — fine for the
   known test.
4. **Signedness/endianness**: use `u8`/`u32` + `to_be_bytes` to avoid the
   C signed-`char` shift bug; byte order must stay big-endian.
5. **Ownership**: `encode` output is owned (`Vec<u8>`); `Message` borrows it,
   so the buffer must outlive the message — natural in Rust, but a test that
   drops the buffer before reading args will not compile (a feature, not a bug).
6. **Edition/toolchain**: use `edition = "2021"` (broadly compatible); avoid
   2024-only features. No external crates, so no version-pinning risk.
7. **Test harness**: `cargo test` runs `tests/*.rs` as integration tests;
   ensure the crate name in `use amp::...` matches the package name `amp`.

## 5. Acceptance criteria
- `cargo build` succeeds with no warnings-as-errors issues (keep `-D warnings`
  clean: no unused imports, no dead code — `VERSION` is `pub` so fine).
- `cargo test` passes: the roundtrip test asserts version == 1, argc == 3,
  and the three decoded arguments equal `some`, `stuff`, `here`.
- Wire bytes produced by `encode` are byte-identical to the C encoder for
  the same argv (header nibble layout + u32be lengths).
