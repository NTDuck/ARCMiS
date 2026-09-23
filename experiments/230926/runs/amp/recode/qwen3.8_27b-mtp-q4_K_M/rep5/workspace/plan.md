# AMP — C → Rust Translation Plan

Translate the ~100-line C AMP library into a single-crate Rust library with no
external dependencies. Target test command: `cargo test`.

## Phase 1 — Fragment extraction (source inventory)

### `src/amp.h` (public API)
| Symbol | Kind | Notes |
|---|---|---|
| `AMP_VERSION` | macro/const | value `1` |
| `amp_t` | struct | fields `version`, `argc`, `buf` (cursor) |
| `amp_encode` | prototype | `char *amp_encode(char **argv, int argc)` |
| `amp_decode` | prototype | `void amp_decode(amp_t *msg, char *buf)` |
| `amp_decode_arg` | prototype | `char *amp_decode_arg(amp_t *msg)` |

### `src/amp.c` (implementation)
| Symbol | Kind | Notes |
|---|---|---|
| `read_u32_be` | static fn | hand-rolled big-endian u32 read |
| `write_u32_be` | static fn | hand-rolled big-endian u32 write |
| `amp_decode` | fn | parses 1-byte header, sets cursor |
| `amp_decode_arg` | fn | reads u32be len + data, malloc copy, advances cursor |
| `amp_encode` | fn | builds header + per-arg (len + data) buffer |

### `tests/test.c`
| Symbol | Kind | Notes |
|---|---|---|
| `main` | fn | encode 3 args, assert header + decoded args |

## Phase 2 — Name mapping (C → Rust)

| Source (C) | Target (Rust) | Reason for change |
|---|---|---|
| `AMP_VERSION` | `VERSION` | Rust consts are `SCREAMING_SNAKE`; drop the `AMP_` prefix (crate is `amp`) |
| `amp_t` | `AmpMessage` | Rust structs are `CamelCase`; descriptive name |
| `amp_t.version` | `AmpMessage.version` | field name preserved |
| `amp_t.argc` | `AmpMessage.argc` | field name preserved |
| `amp_t.buf` | `AmpMessage.buf` | field preserved, now `&'a [u8]` (private) |
| `amp_encode` | `encode` | free function; crate name supplies the `amp::` namespace |
| `amp_decode` | `AmpMessage::decode` | becomes an associated constructor on the struct |
| `amp_decode_arg` | `AmpMessage::decode_arg` | becomes a `&mut self` method |
| `read_u32_be` | *(removed)* | replaced by `u32::from_be_bytes` |
| `write_u32_be` | *(removed)* | replaced by `u32::to_be_bytes` |
| `main` (test) | `encode_decode_three_args` | Rust `#[test]` fn, descriptive name |

Type/semantic changes (documented, strictly safer):
- `char *` buffers → `Vec<u8>` / `&[u8]` (RAII, no manual `free`).
- `NULL` return → `Option` (`encode`, `decode`, `decode_arg`).
- `argc > 15` → `encode` returns `None` (C silently corrupted the header).
- truncated input → `decode_arg` returns `None` (C had UB).
- `short` version/argc → `u8`.

## Phase 3 — Skeleton (already written)

- `Cargo.toml` — package `amp`, edition 2021, no deps, empty `[workspace]` to
  stay standalone.
- `src/lib.rs` — `VERSION`, `AmpMessage<'a>` with `decode`/`decode_arg`,
  free `encode`, plus `#[cfg(test)]` unit-test stubs. All bodies are
  `unimplemented!()`; the file compiles.
- `tests/test.rs` — integration test stub porting `tests/test.c`.

## Phase 4 — Implementation plan

### Part A — source files (bottom-up dependency order)
1. **`src/lib.rs`** — the only source unit. Implement in this order within the
   file (each depends only on std + the const above it):
   a. `VERSION` const (already present).
   b. `AmpMessage::decode` — parse header byte, set `version`/`argc`, set
      cursor to `buf+1`. Depends on: `VERSION` (optional), std.
   c. `AmpMessage::decode_arg` — read u32be length via `u32::from_be_bytes`,
      bounds-check, copy data into `Vec<u8>`, advance cursor. Depends on:
      `AmpMessage` struct.
   d. `encode` — validate `argc <= 15`, build header byte
      (`VERSION << 4 | argc`), append per-arg `u32::to_be_bytes` length + raw
      bytes. Depends on: `VERSION`, std.
   e. `#[cfg(test)] mod tests` — fill the six unit-test stubs (0 args, 15 args,
      16 args → `None`, truncated → `None`, binary round-trip, empty arg).
      Depends on: `encode`, `AmpMessage::decode`, `AmpMessage::decode_arg`.

### Part B — test files (bottom-up dependency order)
1. **`tests/test.rs`** — fill `encode_decode_three_args`: encode
   `["some","stuff","here"]`, assert `version == 1` and `argc == 3`, decode the
   three args and assert each value. Depends on: `amp::encode`,
   `amp::AmpMessage::decode`, `amp::AmpMessage::decode_arg` (Part A).

### Verification
- `cargo build` — library compiles.
- `cargo test` — runs `tests/test.rs` (integration) + `src/lib.rs` unit tests.
