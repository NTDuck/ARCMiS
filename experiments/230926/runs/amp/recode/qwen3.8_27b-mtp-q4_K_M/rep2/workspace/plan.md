# AMP — C → Rust Translation Plan

Source: `clibs/amp` (C, ~100 lines, stdlib-only). Target: single-crate Rust
library, std-only, tested with `cargo test`. Wire format must be preserved
byte-for-byte (1 header byte `version<<4 | argc`, then per-argument
4-byte big-endian length + raw data).

## Fragment extraction (source → target)

| Source fragment | Kind | Target fragment |
|---|---|---|
| `AMP_VERSION` (amp.h) | macro const | `pub const VERSION: u8` |
| `amp_t` (amp.h) | struct | `pub struct Message<'a>` |
| `amp_encode` (amp.c) | function | `pub fn encode(argv: &[&str]) -> Option<Vec<u8>>` |
| `amp_decode` (amp.c) | function | `Message::decode(buf: &'a [u8]) -> Self` |
| `amp_decode_arg` (amp.c) | function | `Message::decode_arg(&mut self) -> Option<String>` |
| `read_u32_be` (amp.c) | static helper | dropped — `u32::from_be_bytes` |
| `write_u32_be` (amp.c) | static helper | dropped — `u32::to_be_bytes` |
| `main` (tests/test.c) | test | `round_trip` + `rejects_too_many_args` in `tests/test.rs` |

## Name mapping

| C name | Rust name | Reason |
|---|---|---|
| `AMP_VERSION` | `VERSION` | Rust const convention (no ALL_CAPS prefix); `u8` instead of macro |
| `amp_t` | `Message` | Rust type naming; `&'a [u8]` cursor replaces `char *buf` |
| `amp_encode` | `encode` | Crate name `amp` already scopes the name |
| `amp_decode` | `Message::decode` | Associated constructor-style fn on the struct |
| `amp_decode_arg` | `Message::decode_arg` | Method on the message (cursor state) |
| `read_u32_be` / `write_u32_be` | — | Replaced by std `u32::from_be_bytes` / `to_be_bytes` |
| `main` (test) | `round_trip` | `#[test]` fn; no `main` needed under `cargo test` |
| `msg->version` / `msg->argc` | `msg.version` / `msg.argc` | `short` → `u8` (4-bit wire field) |

## Skeleton status

- `Cargo.toml` — package `amp`, edition 2021, no deps, empty `[workspace]`
  table so the crate stays standalone when nested in another workspace.
- `src/lib.rs` — `VERSION`, `Message` (fields `version`, `argc`, private
  `buf`), `Message::decode`, `Message::decode_arg`, `encode`; all bodies
  `unimplemented!()` with TODO comments. Compiles (`cargo check --tests` OK).
- `tests/test.rs` — `round_trip`, `rejects_too_many_args` stubs with the
  intended assertions in comments.

## Part A — source files (bottom-up dependency order)

1. **`src/lib.rs`** — the only library unit; no intra-crate dependencies.
   Implement in this order inside the file:
   1. `encode(argv: &[&str]) -> Option<Vec<u8>>`:
      - return `None` if `argv.len() > 15`;
      - header byte `VERSION << 4 | argc as u8`;
      - per arg: `len.to_be_bytes()` then the raw bytes
        (`str::len` / `as_bytes`, `Vec::extend_from_slice`).
   2. `Message::decode(buf: &'a [u8]) -> Self`:
      - `version = buf[0] >> 4`, `argc = buf[0] & 0xf`, `buf = &buf[1..]`.
   3. `Message::decode_arg(&mut self) -> Option<String>`:
      - need ≥ 4 bytes for the length, else `None`;
      - `u32::from_be_bytes` on the first 4 bytes;
      - need ≥ `len` bytes remaining, else `None`;
      - `String::from_utf8_lossy` (or `from_utf8` mapped to `None`) on the
        slice, advance cursor by `4 + len`.
   Wire-format guard: first encoded byte for 3 args must be `0x13`.

## Part B — test files (bottom-up dependency order)

1. **`tests/test.rs`** — depends only on `src/lib.rs` (Part A item 1).
   1. `round_trip`: encode `["some", "stuff", "here"]`, assert
      `version == VERSION`, `argc == 3`, three `decode_arg()` calls return
      `Some("some")`, `Some("stuff")`, `Some("here")`, fourth returns `None`.
   2. `rejects_too_many_args`: 16 args → `encode` returns `None`.
   Verify with `cargo test`.
