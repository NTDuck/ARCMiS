# AMP (C → Rust) Translation Plan

Source: minimal C implementation of the AMP wire format (`src/amp.c`, `src/amp.h`,
`tests/test.c`, `Makefile`). Target: zero-dependency Rust crate `amp`
(edition 2021), test command `cargo test`.

Design rationale: see `design.md`. Skeleton files (compilable stubs) are already
in place: `Cargo.toml`, `src/lib.rs`, `tests/test.rs`.

## Phase 1 — Fragment extraction (source inventory)

| Source file | Fragment | Kind | Notes |
|---|---|---|---|
| `src/amp.h` | `AMP_VERSION` | macro/const | value 1 |
| `src/amp.h` | `amp_t` | struct | `short version; short argc; char *buf;` |
| `src/amp.h` | `amp_encode` | prototype | `char *(char **argv, int argc)` |
| `src/amp.h` | `amp_decode` | prototype | `void (amp_t *, char *)` |
| `src/amp.h` | `amp_decode_arg` | prototype | `char *(amp_t *)` |
| `src/amp.c` | `read_u32_be` | static fn | big-endian u32 read |
| `src/amp.c` | `write_u32_be` | static fn | big-endian u32 write |
| `src/amp.c` | `amp_decode` | fn | header parse, cursor advance |
| `src/amp.c` | `amp_decode_arg` | fn | u32be len + malloc/memcpy, cursor advance |
| `src/amp.c` | `amp_encode` | fn | length calc, malloc, header + args |
| `tests/test.c` | `main` | fn | round-trip test, asserts, prints `ok` |
| `Makefile` | build rules | build | replaced by `cargo test` |

## Phase 2 — Name mapping (C → Rust)

| C symbol | Rust symbol | Reason for change |
|---|---|---|
| `AMP_VERSION` | `VERSION` | Rust const convention: no module-prefix macro style; `amp::VERSION` already carries the crate name |
| `amp_t` | `Message` | Rust struct naming (UpperCamelCase); `amp_t` is a C typedef tag |
| `amp_t.version` | `Message.version` | preserved (field name kept) |
| `amp_t.argc` | `Message.argc` | preserved (field name kept) |
| `amp_t.buf` | `Message.cursor` | renamed: it is a borrow cursor, not an owned buffer; private field in Rust |
| `amp_decode(amp_t*, char*)` | `Message::decode(&'a [u8]) -> Self` | C out-param struct becomes a returned value; free function becomes associated fn |
| `amp_decode_arg(amp_t*)` | `Message::decode_arg(&mut self) -> Option<&'a [u8]>` | C `NULL`-on-failure becomes `Option`; malloc'd copy becomes zero-copy borrow |
| `amp_encode(char**, int)` | `encode(&[&str]) -> Result<Vec<u8>, AmpError>` | C `char**` + count becomes a slice; `NULL`-on-OOM becomes `Result` (argc > 15 is the real error) |
| `read_u32_be` / `write_u32_be` | `u32::from_be_bytes` / `u32::to_be_bytes` (inline) | std intrinsics replace the static helpers; no separate functions needed |
| `main` (tests/test.c) | `#[test] fn test()` in `tests/test.rs` | C test harness replaced by `cargo test` |
| — (new) | `AmpError::ArgcTooLarge` | new: surfaces the protocol's 4-bit argc limit that C silently masks |

## Phase 3 — Skeleton (done)

Compilable stubs written and verified with `cargo check --all-targets`:

- `Cargo.toml` — package `amp`, edition 2021, no dependencies, empty
  `[workspace]` table (keeps the crate standalone when nested in a parent
  Cargo workspace).
- `src/lib.rs` — `VERSION`, `AmpError` (full impl: `Display`/`Error`),
  `Message<'a>` with `decode`/`decode_arg` stubs (`unimplemented!`),
  `encode` stub, plus three edge-case unit-test stubs.
- `tests/test.rs` — `test()` mirroring `tests/test.c` (fully written; it
  exercises the stubs and will pass once the lib is implemented).

## Phase 4 — Implementation plan

### Part A — source files, bottom-up dependency order

1. **`src/lib.rs`** (single unit; internal order within the file):
   1. `AmpError` — no dependencies (already complete in skeleton).
   2. `Message::decode` — depends on `VERSION` semantics only; parse header
      byte (`version = buf[0] >> 4`, `argc = buf[0] & 0xf`), set
      `cursor = &buf[1..]`. Decide behavior on empty input (panic with a
      clear message, matching C's undefined read of `buf[0]` — document it).
   3. `Message::decode_arg` — depends on `Message` layout; read u32be length
      via `u32::from_be_bytes`, bounds-check against remaining cursor
      (`len as usize <= cursor.len()`), return `&cursor[..len]`, advance
      cursor by `4 + len`; `None` on truncation.
   4. `encode` — depends on `VERSION`, `AmpError`; `Err(AmpError::ArgcTooLarge)`
      when `argv.len() > 15`; else build `Vec<u8>` with
      `with_capacity(1 + sum(4 + len))`, push header byte
      `VERSION << 4 | argc`, then per arg `to_be_bytes` length + bytes.
      Happy path must be byte-identical to C (argc ≤ 15).
   5. Unit tests in `#[cfg(test)] mod tests` — fill the three stubs:
      `encode_empty_argv` (`encode(&[]) == Ok(vec![0x10])`),
      `round_trip_empty_string_arg`, `decode_arg_truncated_returns_none`.
   - Verify: `cargo test` (lib unit tests) and `cargo check --all-targets`.

### Part B — test files, bottom-up dependency order

1. **`tests/test.rs`** — depends on `src/lib.rs` (Part A item 1).
   Already written to mirror `tests/test.c` exactly (encode
   `["some","stuff","here"]`, assert `version == 1`, `argc == 3`, decode and
   compare the three args). No changes expected; run `cargo test` to confirm
   it passes against the implemented lib. If any assertion diverges from C
   semantics, fix the lib, not the test.

### Final verification

- `cargo test` — all tests green (integration `test` + 3 unit tests).
- `cargo build --release` — clean build, no warnings.
- Wire-format spot check (optional): `encode(&["some","stuff","here"])`
  must equal `0x13 00 00 00 04 73 6f 6d 65 00 00 00 05 73 74 75 66 66 00 00 00 04 68 65 72 65`.
