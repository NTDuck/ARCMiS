# AMP — C → Rust Translation Design

## 1. Source Project Research

### What it is
`amp` is a tiny C library implementing the "Abstract Message Protocol"
(AMP), originally from `node-amp`. It encodes an argv-style message into a
binary buffer and decodes it back. MIT licensed, ~100 lines of C.

### File inventory
| File | Role |
|---|---|
| `src/amp.h` | Public API: `AMP_VERSION` (1), `amp_t` struct, prototypes |
| `src/amp.c` | Implementation: u32be read/write, `amp_decode`, `amp_decode_arg`, `amp_encode` |
| `tests/test.c` | Single test: encode 3 args, decode header + args, assert values |
| `Makefile` | gcc build with coverage flags; `make test` runs the binary |
| `package.json` | clibs metadata (name `amp`, src `amp.c`/`amp.h`) |

### Public API (C)
```c
#define AMP_VERSION 1

typedef struct { short version; short argc; char *buf; } amp_t;

char *amp_encode(char **argv, int argc);   // malloc'd buffer, caller frees
void  amp_decode(amp_t *msg, char *buf);   // parses 1-byte header, sets msg->buf cursor
char *amp_decode_arg(amp_t *msg);          // reads u32be len + data, malloc'd copy, advances cursor
```

### Wire format
```
------------+------------+------------+ ...
| <ver/argc> | <length>   | <data>     | additional args
| 1 byte     | 4 bytes BE | len bytes  |
------------+------------+------------+
```
- Byte 0: high nibble = protocol version (1), low nibble = argc (0–15).
- Each argument: 4-byte big-endian length followed by raw bytes.

### Behavioral notes / edge cases
- `argc` is limited to 4 bits (0–15); the C code does not validate this.
- `amp_decode_arg` does no bounds checking (reads past buffer on malformed input).
- `amp_encode`/`amp_decode_arg` return `NULL` on allocation failure.
- Decoded args are raw byte copies (not NUL-terminated); length comes from the header.
- No third-party dependencies — pure C standard library (`string.h`, `stdlib.h`, `stdint.h`).

## 2. Third-Party Library Analysis

The source project has **zero third-party dependencies** (only libc).
Therefore the Rust translation needs **no external crates** — everything is
covered by the standard library:

| C dependency | Rust counterpart |
|---|---|
| `string.h` (`strlen`, `memcpy`) | `str::len`, `slice::copy_from_slice` (std) |
| `stdlib.h` (`malloc`) | `Vec<u8>` / `Box<[u8]>` (std) |
| `stdint.h` (u32be helpers) | `u32::from_be_bytes` / `u32::to_be_bytes` (std) |
| `assert.h` (tests) | `assert!` / `cargo test` (std) |

## 3. Target Project Design (Rust)

### Crate layout
```
amp/
├── Cargo.toml          # name = "amp", edition 2021, no dependencies
├── src/
│   └── lib.rs          # the whole library (small enough for one file)
└── tests/
    └── test.rs         # port of tests/test.c (integration test)
```

### `Cargo.toml`
```toml
[package]
name = "amp"
version = "0.0.1"
edition = "2021"
description = "Abstract Message Protocol"
license = "MIT"

[dependencies]
# none
```

### Public API (Rust)
Idiomatic translation: slices instead of `char*`, `Vec<u8>` instead of
malloc'd buffers, `Option`/`Result` instead of `NULL`.

```rust
pub const VERSION: u8 = 1;

/// A decoded AMP message with a cursor over the remaining argument bytes.
pub struct AmpMessage {
    pub version: u8,
    pub argc: u8,
    buf: &[u8],   // cursor: remaining undecoded argument bytes
}

impl AmpMessage {
    /// Decode the 1-byte header in `buf`, leaving the cursor at the args.
    pub fn decode(buf: &[u8]) -> Option<AmpMessage>;

    /// Decode the next argument (u32be length + data), advancing the cursor.
    /// Returns None if the buffer is truncated.
    pub fn decode_arg(&mut self) -> Option<Vec<u8>>;
}

/// Encode an argv into an AMP message buffer.
/// Returns None if argc > 15 (does not fit in the header nibble).
pub fn encode(argv: &[&[u8]]) -> Option<Vec<u8>>;
```

Design decisions:
- **`encode(argv: &[&[u8]])`** — generic over bytes, works for `&str`
  (via `.as_bytes()`) and arbitrary binary args, matching the C API which
  copies raw bytes. `Option<Vec<u8>>` for the argc>15 case (C silently
  truncated; we reject).
- **`AmpMessage::decode(buf: &[u8]) -> Option<AmpMessage>`** — takes a
  borrow, no ownership transfer (C's `amp_t` just held a pointer).
- **`decode_arg(&mut self) -> Option<Vec<u8>>`** — returns an owned
  `Vec<u8>` (equivalent of the C malloc'd copy), advances the cursor.
  Bounds-checked: returns `None` on truncated input instead of UB.
- **u32be** via `u32::from_be_bytes` / `u32::to_be_bytes` — replaces the
  hand-rolled `read_u32_be`/`write_u32_be`.
- `version`/`argc` are `u8` (C used `short`; values fit in a byte).

### Tests
- `tests/test.rs`: direct port of `tests/test.c` — encode
  `["some", "stuff", "here"]`, decode header (assert version == 1,
  argc == 3), decode the three args and assert their values.
- Additional unit tests in `lib.rs` (`#[cfg(test)]`):
  - round-trip with 0 args (argc = 0),
  - round-trip with 15 args (max),
  - `encode` with 16 args returns `None`,
  - `decode_arg` on a truncated buffer returns `None`,
  - binary (non-UTF8) argument round-trip,
  - empty-string argument round-trip.

### Build / test
- `cargo build` compiles the library.
- `cargo test` runs both the integration test (`tests/test.rs`) and the
  unit tests — replaces `make test`.

## 4. Translation Risks

1. **argc overflow**: C silently packs `argc` into 4 bits (values > 15
   corrupt the version nibble). Rust version returns `None` — a behavior
   change, but strictly safer. Documented in the API.
2. **NULL vs Option**: C callers check for `NULL`; Rust callers match on
   `Option`. Any FFI consumer would need adaptation, but the target is a
   native Rust crate, so this is fine.
3. **Bounds checking**: C `amp_decode_arg` reads past the buffer on
   malformed input (UB). Rust returns `None` — tests must not assume the
   C behavior on truncated input.
4. **Ownership model**: C transfers ownership of malloc'd buffers to the
   caller (who must `free`). Rust `Vec<u8>` is RAII — no leak risk, but
   the "free by user" contract disappears; `decode_arg` returns a fresh
   `Vec<u8>` each call, matching the C copy semantics.
5. **NUL termination**: C args are raw byte copies (not C strings); the
   Rust `Vec<u8>` preserves this exactly — no `String` conversion, so no
   UTF-8 assumption is introduced.
6. **Coverage flags**: the Makefile's `-fprofile-arcs -ftest-coverage`
   have no direct `cargo test` equivalent; if coverage is needed,
   `cargo-llvm-cov` (dev-only tool, not a dependency) is the idiomatic
   choice. Not required for the translation.
