# AMP — C → Rust Translation Design

## 1. Source Project Research

### Overview
`amp` is a tiny C library implementing the "Abstract Message Protocol" (AMP),
originally from `clibs/amp` (TJ Holowaychuk, MIT). It encodes/decodes an argv
array into a compact binary wire format. The whole library is ~100 lines.

### File inventory
| File | Role |
|---|---|
| `src/amp.h` | Public API: `amp_t` struct, `amp_encode`, `amp_decode`, `amp_decode_arg`, `AMP_VERSION` |
| `src/amp.c` | Implementation: big-endian u32 helpers, header decode, arg decode (malloc'd copies), encode |
| `tests/test.c` | Round-trip test: encode 3 args, decode header, decode each arg, `assert` equality, print `ok` |
| `Makefile` | gcc build with `-Wall -Wextra -O2` + gcov coverage flags; `make test` runs `./test.out` |
| `package.json` | clibs metadata only (name `amp`, src `amp.c`/`amp.h`) — not a real npm package |
| `Readme.md` | Usage example + protocol description |

### Public API (C)
```c
#define AMP_VERSION 1

typedef struct {
  short version;   // protocol version (1)
  short argc;      // number of arguments (0..15, 4 bits in header)
  char *buf;       // cursor into the payload, advanced by amp_decode_arg
} amp_t;

char *amp_encode(char **argv, int argc);   // malloc'd buffer, NULL on OOM
void  amp_decode(amp_t *msg, char *buf);   // parses 1-byte header, sets msg->buf = buf+1
char *amp_decode_arg(amp_t *msg);          // malloc'd copy of next arg, NULL on OOM; advances msg->buf
```

### Wire format (must be preserved byte-for-byte)
```
------------+----------+------------+----------+------------+
| <ver/argc> | <length> | <data>     | <length> | <data>     | ...
------------+----------+------------+----------+------------+
   1 byte        4 bytes (u32 BE)   len bytes
```
- Byte 0: `version << 4 | argc`. Version is 1, so high nibble = `0x1`.
- `argc` is limited to 4 bits → **max 15 arguments** (implicit constraint in C).
- Each argument: 4-byte big-endian length, then raw bytes.
- No framing, no checksum, no null terminators in the payload.

### Behavior notes / C quirks to carry over or fix
1. `amp_decode_arg` does **no bounds checking** — reading past the buffer is UB in C.
2. `amp_encode` does not validate `argc <= 15`; values > 15 silently corrupt the header.
3. `argc` is stored in a `short` (16-bit) but only 4 bits are transmitted.
4. Ownership: `amp_encode` and `amp_decode_arg` return malloc'd memory the caller must free.
5. No third-party dependencies; pure stdlib (`string.h`, `stdlib.h`, `stdint.h`).
6. Test is a single round-trip with args `["some", "stuff", "here"]`.

## 2. Third-Party Library Analysis

The C project has **zero third-party dependencies** (only libc). Therefore the
Rust translation needs **no external crates** — everything is expressible with
the standard library:

| C dependency | Rust counterpart | Notes |
|---|---|---|
| `string.h` (`strlen`, `memcpy`) | `str::len`, `slice::copy_from_slice` / `Vec::extend_from_slice` | std |
| `stdlib.h` (`malloc`) | `Vec<u8>` / `String` (allocator-managed) | std |
| `stdint.h` (`uint32_t`) | `u32` + `to_be_bytes`/`from_be_bytes` | std |
| `assert.h` (tests) | `assert!` / `assert_eq!` | std |

No `Cargo.toml` dependencies required.

## 3. Target Project Design (Rust)

### Crate layout
```
amp/
├── Cargo.toml          # [package] name = "amp", edition = "2021", no deps
├── src/
│   └── lib.rs          # the whole library (mirrors src/amp.c + src/amp.h)
└── tests/
    └── test.rs         # round-trip test (mirrors tests/test.c)
```

### API design (`src/lib.rs`)
Idiomatic Rust mapping of the C API, preserving semantics but using Rust
ownership instead of manual malloc/free:

```rust
pub const VERSION: u8 = 1;

/// A decoded AMP message with a cursor into the payload.
/// Mirrors C `amp_t`.
pub struct Message<'a> {
    pub version: u8,
    pub argc: u8,
    buf: &'a [u8],   // remaining payload (cursor)
}

impl<'a> Message<'a> {
    /// Decode the 1-byte header from `buf`. Mirrors `amp_decode`.
    pub fn decode(buf: &'a [u8]) -> Self { ... }

    /// Decode the next argument, advancing the cursor.
    /// Mirrors `amp_decode_arg`; returns `None` if the buffer is
    /// exhausted/truncated (C returned NULL only on OOM; here we also
    /// guard against malformed input instead of UB).
    pub fn decode_arg(&mut self) -> Option<String> { ... }
}

/// Encode an argv slice into the AMP wire format.
/// Mirrors `amp_encode(char **argv, int argc)`.
/// Returns `None` if `argv.len() > 15` (4-bit argc field) — the C
/// version silently corrupted the header; we make it explicit.
pub fn encode(argv: &[&str]) -> Option<Vec<u8>> { ... }
```

Design decisions:
- **`encode` returns `Option<Vec<u8>>`** (or `Result<Vec<u8>, Error>`):
  `Vec` replaces the malloc'd buffer; the `Option` covers the new
  argc-range validation. `Vec<u8>` is the natural owned byte buffer.
- **`Message::decode` takes `&[u8]`** (borrowed) instead of a raw pointer —
  no ownership transfer, no free required.
- **`decode_arg` returns `Option<String>`**: the C version returned a malloc'd
  copy the caller freed; `String` is the idiomatic owned copy. `None` signals
  truncation instead of C's undefined behavior. (Alternative: return `&[u8]`
  borrowing from the message — but `String` matches the C ownership model
  more closely and is what the test expects.)
- **Big-endian u32**: use `u32::to_be_bytes()` / `u32::from_be_bytes()`
  instead of hand-rolled shift helpers.
- **`argc` as `u8`**: the C `short` is overkill; the field is 4 bits on the
  wire. `u8` is sufficient and honest.
- Keep the module flat (single `lib.rs`) — the source is one file; no need
  to invent a module tree.

### Test design (`tests/test.rs`)
Mirror `tests/test.c` exactly, plus a couple of robustness tests:
```rust
use amp::{encode, Message, VERSION};

#[test]
fn round_trip() {
    let args = ["some", "stuff", "here"];
    let buf = encode(&args).unwrap();
    let mut msg = Message::decode(&buf);
    assert_eq!(VERSION, msg.version);
    assert_eq!(3, msg.argc);
    assert_eq!(Some("some".to_string()), msg.decode_arg());
    assert_eq!(Some("stuff".to_string()), msg.decode_arg());
    assert_eq!(Some("here".to_string()), msg.decode_arg());
    assert_eq!(None, msg.decode_arg()); // exhausted
}

#[test]
fn rejects_too_many_args() {
    let args: Vec<&str> = (0..16).map(|i| format!("a{i}").leak()).collect();
    assert!(encode(&args).is_none());
}
```
Run with `cargo test` (the required test command).

### Cargo.toml
```toml
[package]
name = "amp"
version = "0.0.1"
edition = "2021"
description = "Abstract Message Protocol"
license = "MIT"

[lib]
name = "amp"
path = "src/lib.rs"
```

## 4. Translation Risks

1. **Wire-format fidelity** — the byte layout (1 header byte, u32 BE lengths)
   must match exactly for interoperability with node-amp and the C version.
   Mitigation: keep the format constants identical; the round-trip test plus
   a byte-level assertion on the encoded output (e.g. first byte == `0x13`
   for 3 args) guards this.
2. **argc > 15** — C silently corrupts the header; Rust `encode` will return
   `None`. This is a deliberate behavior change (safer), but callers relying
   on the old behavior would notice. Document it.
3. **Truncated/malformed input** — C `amp_decode_arg` reads out of bounds
   (UB); Rust returns `None` on truncation. Safer, but the API shape changes
   from `char*` to `Option<String>`.
4. **Ownership model change** — C callers `free()` results; Rust callers do
   not. Any FFI consumer of the original C API would need a separate
   `#[no_mangle]` shim (not required here since the target is a pure Rust
   crate, but noted if C interop is ever needed).
5. **`short` vs `u8` for version/argc** — semantically identical for the
   4-bit wire fields; no risk.
6. **No external deps** — eliminates version/API-drift risk entirely; the
   translation is std-only and stable across Rust versions.
