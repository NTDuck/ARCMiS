# AMP: C → Rust Translation Plan

Source: clibs `amp` (C, zero third-party deps). Target: single-module Rust
crate `amp`, edition 2021, no external crates. Test command: `cargo test`.

## 1. Fragment extraction (source inventory)

### `src/amp.h` (public API)
| Symbol | Kind | Role |
|---|---|---|
| `AMP_VERSION` | macro const (=1) | protocol version |
| `amp_t` | struct | `version`, `argc`, `buf` cursor |
| `amp_encode` | prototype | encode argv → malloc'd buffer |
| `amp_decode` | prototype | decode header into `amp_t` |
| `amp_decode_arg` | prototype | decode next arg (malloc'd copy) |

### `src/amp.c` (implementation)
| Symbol | Kind | Role |
|---|---|---|
| `read_u32_be` | static fn | read u32 big-endian from buffer |
| `write_u32_be` | static fn | write u32 big-endian into buffer |
| `amp_decode` | fn | header decode: `version = buf[0] >> 4`, `argc = buf[0] & 0xf`, cursor = `buf + 1` |
| `amp_decode_arg` | fn | read u32be length, malloc+memcpy arg, advance cursor |
| `amp_encode` | fn | size = `1 + Σ(4 + len)`; header byte `(AMP_VERSION << 4) \| argc`; per arg: u32be len + raw bytes |

### `tests/test.c`
| Symbol | Kind | Role |
|---|---|---|
| `main` | fn | encode `{"some","stuff","here"}`, assert version==1, argc==3, args equal expected strings |

### Non-code files
`Makefile` (build/run), `package.json` (clibs metadata), `Readme.md` (docs) —
no translation needed; `Cargo.toml` replaces the Makefile/package.json role
(skeleton already written).

## 2. Name mapping (C → Rust)

| C symbol | Rust symbol | Reason for change |
|---|---|---|
| `AMP_VERSION` | `VERSION` | Rust consts are `SCREAMING_SNAKE` without a prefix; `AMP_` prefix is redundant in a crate named `amp` |
| `amp_t` | `Message<'a>` | C `typedef` struct → Rust struct; lifetime parameter added because it borrows the input buffer (zero-copy) |
| `amp_t.version` / `amp_t.argc` | `Message.version` / `Message.argc` | field names preserved; type `short` → `u8` |
| `amp_t.buf` (cursor) | `Message.rest` (private) + `consumed` (private) | cursor kept as private `&'a [u8]`; `consumed` counter added so `arg()` returns `None` after `argc` args (C had no such bound) |
| `amp_encode` | `encode` | crate name `amp` replaces the `amp_` prefix |
| `amp_decode` | `decode` | same |
| `amp_decode_arg` | `Message::arg` | C free function on `amp_t*` → method on `Message`; return `char*` (malloc'd copy) → `Option<&'a [u8]>` (zero-copy borrow) |
| `read_u32_be` | `read_u32_be` (private) | name preserved; now `&[u8] → u32`, unsigned arithmetic fixes the C signed-`char` shift bug |
| `write_u32_be` | `write_u32_be` (private) | name preserved; now `(&mut [u8], u32)` |
| `main` (test) | `encode_decode_roundtrip` | C `main` → Rust `#[test]` fn; descriptive name per Rust convention |
| `NULL` return | `Result<_, Error>` / `Option` | Rust idiom: `Error::TooManyArguments`, `Error::TruncatedBuffer` |
| `malloc`/`free` | `Vec<u8>` / RAII | ownership replaces manual memory management |

## 3. Skeleton (already written, compiles)

- `Cargo.toml` — package `amp`, edition 2021, no deps, empty `[workspace]`
  (isolates the crate from the enclosing repo's cargo workspace).
- `src/lib.rs` — `VERSION`, `Error`, `Message<'a>`, `read_u32_be`,
  `write_u32_be`, `encode`, `decode`, `Message::arg` as stubs with TODO
  comments. `cargo build` and `cargo test --no-run` succeed.
- `tests/test.rs` — `encode_decode_roundtrip` mirroring `tests/test.c` 1:1.

## 4. Implementation plan

### Part A — source files (bottom-up dependency order)

1. **`src/lib.rs`** (single unit; internal order within the file):
   1. `read_u32_be(buf: &[u8]) -> u32` — `u32::from_be_bytes` or manual
      `u8` shifts (no dependencies).
   2. `write_u32_be(buf: &mut [u8], n: u32)` — `u32::to_be_bytes`
      (no dependencies).
   3. `encode<I, T>(args: I) -> Result<Vec<u8>, Error>` — depends on
      `VERSION`, `Error`, `write_u32_be`. Reject `argc > 15` with
      `Error::TooManyArguments`; capacity `1 + Σ(4+len)`; header byte
      `(VERSION << 4) | argc`; per arg: 4 BE length bytes + raw bytes.
   4. `decode<'a>(buf: &'a [u8]) -> Result<Message<'a>, Error>` — depends on
      `Error`, `Message`. Empty buffer → `Error::TruncatedBuffer`;
      `version = buf[0] >> 4`, `argc = buf[0] & 0xf`, `rest = &buf[1..]`,
      `consumed = 0`.
   5. `Message::arg(&mut self) -> Option<&'a [u8]>` — depends on
      `read_u32_be`. `None` if `consumed >= argc`; need ≥4 bytes for length
      else `None`; advance 4; need `n` more bytes else `None`; return slice,
      advance `n`, `consumed += 1`.
   - Acceptance: `cargo build` clean (no warnings), wire bytes identical to
     the C encoder for the same argv.

### Part B — test files (bottom-up dependency order)

1. **`tests/test.rs`** — depends on the finished `src/lib.rs` API
   (`encode`, `decode`, `VERSION`, `Message::arg`).
   - `encode_decode_roundtrip` (already written, mirrors `tests/test.c`):
     encode `["some","stuff","here"]`, assert `version == 1`,
     `version == VERSION`, `argc == 3`, args equal `b"some"`, `b"stuff"`,
     `b"here"`, then `arg()` is `None`.
   - Optional extra tests (only if time permits; not required): empty argv,
     single arg, binary/empty-string arg, truncated buffer → `None`/`Err`,
     `argc > 15` → `Error::TooManyArguments`.
   - Acceptance: `cargo test` exits 0 with all tests passing.

## 5. Verification

```
cargo build   # no warnings
cargo test    # encode_decode_roundtrip passes
```
