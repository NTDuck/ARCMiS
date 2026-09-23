# AMP (C → Rust) — Brainstorm / Translation Plan

Source analyzed: `src/amp.h`, `src/amp.c`, `tests/test.c`, `Makefile`, `Readme.md`, `package.json`.

AMP is a tiny binary protocol: a 1-byte header (`version << 4 | argc`, version = 1),
followed by `argc` frames of `<u32be length><data>`. The C API is:

```c
char *amp_encode(char **argv, int argc);   // malloc'd buffer, NULL on OOM
void  amp_decode(amp_t *msg, char *buf);   // msg->buf becomes a cursor into buf
char *amp_decode_arg(amp_t *msg);          // malloc'd copy, advances cursor, NULL on OOM
```

---

## 1. Core difficulties of translating this codebase

### 1.1 Ownership / lifetimes — the `amp_t` cursor

`amp_t` holds `char *buf`, a **cursor into a caller-owned buffer**. In C this is
fine (raw pointer, no ownership). In Rust the natural mapping is:

```rust
struct Amp<'a> {
    version: u8,
    argc: u8,
    buf: &'a [u8],   // cursor: a slice that shrinks as args are decoded
}
```

- `decode(buf: &[u8]) -> Amp` (or `Amp::new(buf)`) sets `version = buf[0] >> 4`,
  `argc = buf[0] & 0xf`, and `buf = &buf[1..]`.
- `decode_arg(&mut self) -> Vec<u8>` reads the u32be length from `self.buf`,
  copies that many bytes into a fresh `Vec<u8>`, and advances the cursor with
  `self.buf = &self.buf[4 + len..]`.

Key decisions:

- **`&mut self` for `decode_arg`** — the cursor mutates, so the method must take
  `&mut self`. This is the idiomatic Rust equivalent of the C function mutating
  `msg->buf`.
- **Return `Vec<u8>` (owned copy)** — exactly mirrors the C contract "returns a
  buffer that must be freed by the user". The caller owns the result; no
  lifetime coupling to the message. `String` is *not* correct in general because
  AMP is a binary protocol and payloads may contain invalid UTF-8; `Vec<u8>` is
  the faithful type. (Tests can compare against `b"some"` byte literals.)
- **Bounds checking**: C happily reads out of bounds on a truncated buffer. Rust
  slicing panics on out-of-range. We should *check* the length and the remaining
  slice and return an error (or panic with a clear message) instead of UB. This
  is a strict improvement and does not change behavior on well-formed input.
  (See 1.4 for the error-handling decision.)

### 1.2 The `lens[argc]` VLA in `amp_encode`

C99 variable-length arrays do not exist in Rust. Straightforward replacement:

```rust
let mut lens: Vec<u32> = Vec::with_capacity(argc);
```

or, better, compute the total length in one pass and write directly in a second
pass (the C code already does two passes: one for total length + per-arg lengths,
one for writing). We can keep the two-pass shape with a `Vec<u32>` of lengths,
or recompute `arg.len()` in the second pass (cheap, and avoids the Vec entirely).
Either is fine; the `Vec` keeps the structure closest to the original.

### 1.3 `char *` vs bytes

AMP is a **binary** protocol. All C `char *` / `strlen` / `memcpy` / `strcmp`
must map to Rust byte types:

| C | Rust |
|---|------|
| `char *buf` (message buffer) | `&[u8]` (borrowed) / `Vec<u8>` (owned) |
| `char **argv` | `&[&[u8]]` (or `&[Vec<u8>]`) |
| `strlen(argv[i])` | `arg.len()` — note: this also *fixes* a C limitation, since `strlen` stops at NUL bytes; Rust slices carry their length, so embedded NULs work |
| `malloc` + `memcpy` | `Vec::with_capacity` + `extend_from_slice` |
| `strcmp(a, b) == 0` | `a == b` on `&[u8]` / `Vec<u8>` |
| `read_u32_be` / `write_u32_be` | `u32::from_be_bytes` / `to_be_bytes` (or manual shifts, to stay close to the source) |

### 1.4 Error handling

C returns `NULL` on `malloc` failure. Options in Rust:

1. **`Result<Vec<u8>, AmpError>`** everywhere — most "correct", but `Vec`
   allocation realistically never fails (the allocator aborts), so `Result`
   would be dead code for the OOM case.
2. **Plain `Vec<u8>` returns, no `Result`** — matches Rust idiom: `Vec`
   allocation is infallible from the caller's perspective.

**Decision: use plain `Vec<u8>` / `String` returns for `encode` and
`decode_arg`** (option 2), because the only C error path (OOM) has no Rust
equivalent — `Vec` does not fail. However, `decode_arg` gains a *new* possible
failure in Rust: a truncated/malformed buffer. For that we have two sub-options:

- (a) panic with a descriptive message (simplest; the C code would have been UB
  anyway, so no well-defined behavior is being lost), or
- (b) return `Option<Vec<u8>>` / `Result<Vec<u8>, AmpError>` from `decode_arg`.

**Recommendation: (b) `Result<Vec<u8>, AmpError>` for `decode_arg`** (and
`decode`/`new` returning `Result<Amp, AmpError>` if the buffer is shorter than
1 byte), because a malformed input is a *runtime data* error, not an OOM —
`Result` is the idiomatic Rust tool for that, and it keeps the library usable
from network contexts (the Readme mentions tcp/udp) without panics. `encode`
stays infallible: `fn encode(args: &[&[u8]]) -> Vec<u8>`.

### 1.5 Tests: `assert!` + `strcmp` → Rust tests

`tests/test.c` is a `main()` that encodes `{"some","stuff","here"}`, decodes the
header, asserts `version == 1` and `argc == 3`, then decodes each arg and
`strcmp`s it against the expected value, printing `"ok"`.

Rust mapping (integration test in `tests/test.rs`):

```rust
use amp::{encode, Amp};

#[test]
fn encode_decode_roundtrip() {
    let args: &[&[u8]] = &[b"some", b"stuff", b"here"];
    let buf = encode(args);

    let mut msg = Amp::new(&buf).unwrap();
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 3);

    for expected in args {
        let arg = msg.decode_arg().unwrap();
        assert_eq!(arg.as_slice(), expected);
    }
}
```

- `assert(cond)` → `assert_eq!(a, b)` (better: shows both values on failure).
- `strcmp(a, b) == 0` → `assert_eq!(arg.as_slice(), b"some")` (byte-slice
  equality; no `String` needed).
- `printf("ok")` → dropped; `cargo test` reports pass/fail itself.
- The C `switch (i) { case 0: ... }` becomes a simple loop over the expected
  values — same assertions, less noise.

### 1.6 Build system: Makefile → Cargo

The Makefile compiles `src/amp.c` + `tests/test.c` with `-Wall -Wextra -O2` plus
gcov coverage flags, links `test.out`, and runs it. Mapping:

- `Cargo.toml`: package `amp`, `edition = "2021"`, `license = "MIT"`,
  description/keywords from `package.json` ("Abstract Message Protocol",
  amp/tcp/udp/message/protocol/encode/decode). No dependencies.
- Library target from `src/lib.rs` (re-exporting the `amp` module) — replaces
  `src/amp.c` + `src/amp.h`.
- `tests/test.rs` integration test — replaces `tests/test.c`.
- `cargo test` replaces `make test`; `-Wall -Wextra` is subsumed by
  `#![deny(warnings)]`-free defaults plus clippy if desired. Coverage flags
  (`-fprofile-arcs -ftest-coverage`) are a tooling concern, not part of the
  translation; `cargo-llvm-cov` can reproduce them if needed.
- `package.json` is a clibs manifest; its metadata (name, description, license,
  keywords) is carried into `Cargo.toml`.

---

## 2. Candidate API shapes

### Approach A — struct with cursor + `&mut self` methods (recommended)

```rust
pub struct Amp<'a> {
    pub version: u8,
    pub argc: u8,
    buf: &'a [u8],              // private cursor
}

impl<'a> Amp<'a> {
    pub fn new(buf: &'a [u8]) -> Result<Self, AmpError>;
    pub fn decode_arg(&mut self) -> Result<Vec<u8>, AmpError>;
}

pub fn encode(args: &[&[u8]]) -> Vec<u8>;
```

Pros:
- Mirrors the C structure 1:1 (`amp_t` → `Amp`, `amp_decode` → `Amp::new`,
  `amp_decode_arg` → `decode_arg`, `amp_encode` → free `encode`).
- The cursor is a private `&'a [u8]`; the borrow checker guarantees the cursor
  never outlives the input buffer — the C code has no such guarantee.
- `&mut self` makes "decoding consumes the message" explicit and serial.
- `encode` stays a free function because it has no state — same as C.

Cons:
- Slightly more ceremony than free functions (`Amp::new` vs `amp_decode`).

### Approach B — all free functions, message as a plain data struct

```rust
pub struct Amp { pub version: u8, pub argc: u8 }

pub fn decode(buf: &[u8]) -> Amp;
pub fn decode_arg(buf: &[u8]) -> (Vec<u8>, &[u8]);   // returns next cursor
pub fn encode(args: &[&[u8]]) -> Vec<u8>;
```

Pros:
- Closest textual shape to the C API; no `&mut`.
Cons:
- The cursor must be threaded manually by the caller (`decode_arg` returns a
  new slice), which is awkward and error-prone — the C API hides the cursor in
  the struct precisely to avoid this.
- Returning `(Vec<u8>, &[u8])` tuples is unidiomatic.

### Recommendation

**Approach A.** It preserves the C design (stateful message + stateless
encoder) while letting Rust ownership express the cursor safely. The test maps
naturally onto it, and it is the shape a Rust user would expect for a streaming
decoder.

---

## 3. File layout plan

```
workspace/
├── Cargo.toml          # package "amp", edition 2021, MIT, no deps
├── src/
│   ├── lib.rs          # `pub mod amp; pub use amp::*;` (or put everything here)
│   └── amp.rs          # Amp struct, AmpError, encode, decode helpers,
│                       #   read_u32_be / write_u32_be (private)
├── tests/
│   └── test.rs         # integration test mirroring tests/test.c
├── src/amp.c, src/amp.h, tests/test.c, Makefile, Readme.md, package.json
│                       # (original C sources kept for reference)
└── brainstorm.md       # this document
```

Notes:
- `lib.rs` as the crate root with `amp.rs` as the module mirrors the C
  `amp.h`/`amp.c` split (header = public API, source = implementation).
  Alternatively a single `src/lib.rs` is fine for ~80 lines; either is
  acceptable — prefer `lib.rs` + `amp.rs` to keep the mapping visible.
- `AmpError` can be a simple `#[derive(Debug)]` enum
  (`BufferTooShort`, `TruncatedArgument`) or a `String`-based error; keep it
  minimal.
- Test mapping (C → Rust), line by line:

| C (`tests/test.c`) | Rust (`tests/test.rs`) |
|---|---|
| `char *args[] = {"some","stuff","here"}` | `let args: &[&[u8]] = &[b"some", b"stuff", b"here"];` |
| `char *buf = amp_encode(args, 3)` | `let buf = amp::encode(args);` |
| `amp_t msg = {0}; amp_decode(&msg, buf);` | `let mut msg = Amp::new(&buf).unwrap();` |
| `assert(1 == msg.version)` | `assert_eq!(msg.version, 1);` |
| `assert(3 == msg.argc)` | `assert_eq!(msg.argc, 3);` |
| loop + `switch` + `assert(0 == strcmp("some", arg))` | `for expected in args { assert_eq!(msg.decode_arg().unwrap().as_slice(), expected); }` |
| `printf("ok")` | (dropped — `cargo test` prints the result) |

---

## 4. Edge cases to preserve / handle

1. **`argc > 15` truncation**: the header byte only stores 4 bits of argc
   (`AMP_VERSION << 4 | argc`). C silently truncates (`argc & 0xf` on decode,
   and `argc` written unmasked on encode — so encoding 17 args writes `0x11`
   and decodes back as 1). Preserve the 4-bit packing exactly; optionally
   document/`debug_assert` that `argc <= 15`. Do **not** "fix" it — the
   protocol is what it is.
2. **Empty args list (`argc == 0`)**: `encode(&[])` must produce a single
   header byte `0x10`; decoding it yields `version = 1, argc = 0` and zero
   `decode_arg` calls.
3. **Empty argument (zero-length payload)**: `<u32be 0>` frame; `decode_arg`
   must return an empty `Vec` and advance the cursor by 4. (C's `malloc(0)`
   is implementation-defined; Rust's empty `Vec` is clean.)
4. **Binary payloads / embedded NUL bytes**: C's `strlen`-based encode cannot
   represent them; the Rust `&[u8]`-based encode *can*. Preserve the wire
   format (length-prefixed frames) so both work; this is a superset, not a
   break.
5. **Truncated / malformed buffers**: C reads out of bounds (UB). Rust must
   bounds-check and return `Err` (per §1.4) instead of panicking on the
   length read or the payload copy.
6. **Version field**: decode must accept any 4-bit version (store `buf[0] >> 4`
   as-is); the test asserts `version == 1` for our own encoding.
7. **Big-endian u32 lengths**: keep `read_u32_be`/`write_u32_be` semantics
   exactly (network byte order) — use `from_be_bytes`/`to_be_bytes`.
8. **Cursor exhaustion**: after `argc` successful `decode_arg` calls the
   cursor is at the end of the buffer; further calls should error, not read
   past the end.

---

## Key decisions (summary)

- `Amp<'a>` struct with private `&'a [u8]` cursor; `decode_arg(&mut self)`.
- `encode(args: &[&[u8]]) -> Vec<u8>` — infallible (Vec allocation doesn't fail;
  C's NULL-on-OOM path has no Rust equivalent).
- `decode_arg` / `Amp::new` return `Result<_, AmpError>` for malformed input
  (new, well-defined behavior replacing C's UB).
- Bytes everywhere (`&[u8]`/`Vec<u8>`), no `String` in the API (binary protocol).
- `lens[argc]` VLA → `Vec<u32>` (or recompute lengths in the write pass).
- Layout: `Cargo.toml`, `src/lib.rs` + `src/amp.rs`, `tests/test.rs`;
  `cargo test` replaces the Makefile; package metadata carried from
  `package.json`.
- Preserve the 4-bit argc packing (including its >15 truncation quirk) and the
  exact wire format.
