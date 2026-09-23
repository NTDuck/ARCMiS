# Validation Report — AMP (C → Rust) translation

## Commands run

| Command | Result |
|---|---|
| `cargo test` | ✅ PASS — 8/8 tests green (3 unit + 4 in `tests/argc_too_large.rs` + 1 in `tests/test.rs`), 0 doc-tests |
| `cargo build --release` | ✅ Clean, no warnings |
| `cargo check --all-targets` | ✅ Clean, no warnings |

## Test results

### Unit tests (`src/lib.rs`, `mod tests`)
- `tests::encode_empty_argv` — ok
- `tests::round_trip_empty_string_arg` — ok
- `tests::decode_arg_truncated_returns_none` — ok

### Integration tests (`tests/argc_too_large.rs`)
- `encode_rejects_16_args` — ok
- `encode_rejects_17_args_with_count` — ok
- `encode_accepts_exactly_15_args` — ok
- `argc_too_large_display_names_count` — ok

### Integration tests (`tests/test.rs`, mirrors `tests/test.c`)
- `test` — ok

**Failing tests: none.**

## Function coverage vs. plan

| Plan function | Rust symbol | Test coverage |
|---|---|---|
| `AMP_VERSION` | `VERSION` | ✅ asserted in `tests/test.rs` (`VERSION == 1`) and header check in `argc_too_large.rs` |
| `amp_t` | `Message` | ✅ exercised by all decode tests |
| `amp_decode` | `Message::decode` | ✅ `test.rs`, `round_trip_empty_string_arg`, `decode_arg_truncated_returns_none`, `encode_accepts_exactly_15_args` |
| `amp_decode_arg` | `Message::decode_arg` | ✅ `test.rs`, `round_trip_empty_string_arg`, `decode_arg_truncated_returns_none`, `encode_accepts_exactly_15_args` |
| `amp_encode` | `encode` | ✅ `test.rs`, `encode_empty_argv`, all `argc_too_large.rs` tests (happy path, boundary argc=15, error argc=16/17) |
| `read_u32_be` / `write_u32_be` | inlined via `u32::from_be_bytes` / `u32::to_be_bytes` | ✅ covered indirectly through encode/decode round-trips (no separate functions per plan) |
| `main` (tests/test.c) | `#[test] fn test` in `tests/test.rs` | ✅ `test` |
| (new) `AmpError::ArgcTooLarge` | `AmpError` | ✅ `encode_rejects_16_args`, `encode_rejects_17_args_with_count`, `argc_too_large_display_names_count` |

**Uncovered functions: none.**

## Notes

- Wire-format spot check from the plan holds: `encode(&["some","stuff","here"])` round-trips byte-for-byte through `Message::decode`/`decode_arg` (verified by `tests/test.rs`).
- The new `AmpError::ArgcTooLarge` path (argc > 15) is tested at the boundary (15 accepted, 16/17 rejected with correct count) and its `Display` impl is verified.
- `test_generation` was `false`; no additional tests were written.

## Conclusion

Build succeeds, all 8 tests pass, and every function in the plan has test coverage. The translator's report is accurate.
