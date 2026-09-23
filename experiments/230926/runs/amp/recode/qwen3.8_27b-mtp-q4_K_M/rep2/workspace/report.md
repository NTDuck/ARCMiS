# Validation Report — amp (C → Rust)

## Build
- `cargo build`: success, no warnings.

## Tests (`cargo test`)
- `tests/test.rs::round_trip` — ok
- `tests/test.rs::rejects_too_many_args` — ok
- Result: 2 passed, 0 failed.

## Failures
None.

## Coverage vs plan function list
| Plan function | Coverage |
|---|---|
| `encode` | round_trip, rejects_too_many_args |
| `Message::decode` | round_trip |
| `Message::decode_arg` | round_trip |
| `read_u32_be` / `write_u32_be` | dropped per plan (std `u32::from_be_bytes` / `to_be_bytes`) |

All planned functions have test coverage. No uncovered functions.
