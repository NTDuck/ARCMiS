# Plan: C -> Rust translation of the AMP protocol library

## Source of truth

The authoritative C sources are in the workspace (restored from spec):

- `src/amp.h` — `amp_t` struct, `AMP_VERSION`, prototypes
- `src/amp.c` — `amp_encode`, `amp_decode`, `amp_decode_arg`, u32be helpers
- `tests/test.c` — round-trip test (encode 3 args, decode header + args)
- `Makefile`, `Readme.md`, `package.json` — project metadata

The C sources are kept in the workspace as reference.

## The REAL protocol (corrects the previous wrong plan)

There is **no** Amp value enum (no Null/Bool/Int/Str/Bytes/List/Map), no tag
bytes, no `AmpError` type. The protocol is the uber-simple **argv protocol**:

```
         0        1 2 3 4     <length>    ...
------------+----------+------------+
| <ver/argc> | <length> | <data>     | additional arguments
------------+----------+------------+
```

- Byte 0: header — high nibble = version (`AMP_VERSION = 1`),
  low nibble = argc (0..15).
- Then, per argument: 4-byte **big-endian** length, followed by that many
  raw bytes.
- `amp_decode` sets `version = buf[0] >> 4`, `argc = buf[0] & 0xf`, and
  advances the buffer cursor past the header byte.
- `amp_decode_arg` reads a u32be length at the cursor, copies that many
  bytes into a fresh buffer, and advances the cursor by 4 + len.
- `amp_encode` writes the header byte then each (u32be len + bytes) pair.

## Rust crate design

### Cargo.toml

- package name `amp`, edition 2021, **no dependencies**.

### src/lib.rs — public API

```rust
pub const AMP_VERSION: u8 = 1;

/// Decoded AMP message with a read cursor over the payload.
pub struct Amp {
    pub version: u8,
    pub argc: u8,
    pub buf: Vec<u8>,   // payload after the header byte
    pub pos: usize,     // cursor advanced by amp_decode_arg
}

/// Encode an argv list into an AMP message:
/// header byte (AMP_VERSION << 4 | argc) then per arg: u32be len + bytes.
pub fn amp_encode(argv: &[&[u8]]) -> Vec<u8>;

/// Decode the header in `buf` into `msg`:
/// version = buf[0] >> 4, argc = buf[0] & 0xf, buf = buf[1..].to_vec().
pub fn amp_decode(msg: &mut Amp, buf: &[u8]);

/// Read one argument: u32be len at msg.pos, copy len bytes,
/// advance msg.pos by 4 + len, return the argument bytes.
pub fn amp_decode_arg(msg: &mut Amp) -> Vec<u8>;
```

Notes:

- `amp_decode_arg` returns `Vec<u8>` (raw bytes, matching C's `char *`
  buffer); callers can `String::from_utf8` when they know the arg is text.
- Cursor semantics must mirror C: `pos` advances on every successful
  `amp_decode_arg` call.
- u32be read/write must be byte-exact with the C helpers.

### tests/test.rs — 1:1 port of tests/test.c

```rust
#[test]
fn test() {
    let args: Vec<&[u8]> = vec![b"some", b"stuff", b"here"];

    // encode
    let buf = amp_encode(&args);

    // header
    let mut msg = Amp { version: 0, argc: 0, buf: vec![], pos: 0 };
    amp_decode(&mut msg, &buf);
    assert_eq!(1, msg.version);
    assert_eq!(3, msg.argc);

    // args
    for (i, expected) in [b"some", b"stuff", b"here"].iter().enumerate() {
        let arg = amp_decode_arg(&mut msg);
        assert_eq!(*expected, arg.as_slice());
    }
}
```

(Exact expected values per index, mirroring the C switch/case asserts.)

## Task mapping

- t1: scaffold Cargo.toml + src/lib.rs skeleton with the API above.
- t2: implement `amp_encode`, `amp_decode`, `amp_decode_arg` in src/lib.rs
  byte-exactly per the C code (u32be, nibble packing, cursor advance).
- t3: port tests/test.c to tests/test.rs as above.
- t4: `cargo build` (no warnings) and `cargo test` (all green).
- t5: parity check vs C semantics; confirm public API matches this plan.
