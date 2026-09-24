# AMP C → Rust Translation Plan

Source: C library `clibs/amp` (src/amp.h, src/amp.c, tests/test.c).
Target: single-crate Rust library, no external dependencies, tested with `cargo test`.

## Fragment inventory (source)

| Source symbol | Kind | Target symbol |
|---|---|---|
| `AMP_VERSION` (amp.h) | macro | `VERSION` (pub const u8) |
| `amp_t` (amp.h) | struct | `AmpMessage<'a>` (pub struct) |
| `read_u32_be` (amp.c) | static fn | inlined via `u32::from_be_bytes` |
| `write_u32_be` (amp.c) | static fn | inlined via `u32::to_be_bytes` |
| `amp_decode` (amp.c) | fn | `decode` (pub fn) |
| `amp_decode_arg` (amp.c) | fn | `decode_arg` (pub fn) |
| `amp_encode` (amp.c) | fn | `encode` (pub fn) |
| `main` (tests/test.c) | fn | `#[test] fn roundtrip` (tests/test.rs) |

## Name mapping

- `AMP_VERSION` → `VERSION`: Rust const naming; the crate is already named
  `amp`, so the `AMP_` prefix is dropped (design.md specifies `VERSION`).
- `amp_t` → `AmpMessage`: C struct tag → Rust struct name; `short` fields
  become `u8`; `char *buf` becomes a private `buf: &'a [u8]` cursor.
- `amp_encode` → `encode`, `amp_decode` → `decode`, `amp_decode_arg` →
  `decode_arg`: the `amp_` C prefix is carried by the crate name.
- `read_u32_be` / `write_u32_be` → no standalone functions; replaced by
  `u32::from_be_bytes` / `u32::to_be_bytes` (wire format unchanged).
- C `NULL` failure returns → `Result<_, AmpError>` with variants
  `Truncated`, `BadLength`, `TooManyArgs` (new type, no C counterpart).
- `char *` return values → `Vec<u8>` for `encode` (owned, replaces
  malloc/free), zero-copy `&[u8]` for `decode_arg` (borrows from the
  message instead of per-arg malloc).
- `tests/test.c` `main` → `#[test] fn roundtrip` in `tests/test.rs`.

## Part A — source files (bottom-up dependency order)

1. **`src/lib.rs`** — the entire library in one file (already stubbed,
   compiles with `cargo build`):
   - `VERSION` const (no deps)
   - `AmpError` enum + `Display`/`Error` impls (no deps)
   - `AmpMessage<'a>` struct + `Default` (no deps)
   - `encode(argv: &[&[u8]]) -> Vec<u8>` — port of `amp_encode`:
     panic if `argv.len() > 15`; write header byte `VERSION << 4 | argc`;
     per arg write `u32::to_be_bytes(len)` then the raw bytes.
   - `decode(msg: &mut AmpMessage, buf: &[u8]) -> Result<(), AmpError>` —
     port of `amp_decode`: `Err(Truncated)` if `buf.is_empty()`; else
     `version = buf[0] >> 4`, `argc = buf[0] & 0xf`, cursor = `&buf[1..]`.
   - `decode_arg(msg: &mut AmpMessage) -> Result<&[u8], AmpError>` — port
     of `amp_decode_arg`: need ≥ 4 bytes for the length (`Err(BadLength)`
     otherwise); `len = u32::from_be_bytes`; need ≥ `len` more bytes
     (`Err(BadLength)` otherwise); return the slice and advance the cursor.
     No allocation (zero-copy improvement over C).
   - Unit tests in `#[cfg(test)] mod tests`: `empty_argv`, `single_arg`,
     `binary_arg_with_nul`, `truncated_buffer_errors`,
     `too_many_args_rejected` (`#[should_panic]`).

## Part B — test files (bottom-up dependency order)

1. **`tests/test.rs`** — integration test mirroring `tests/test.c`
   (already stubbed, compiles with `cargo test --no-run`):
   - `#[test] fn roundtrip`: encode `["some", "stuff", "here"]`, decode
     header (assert `version == VERSION`, `argc == 3`), decode all three
     args and assert byte equality.

## Verification

- `cargo build` — library compiles (stubs with `todo!`).
- `cargo test` — after implementation, all unit + integration tests pass.
