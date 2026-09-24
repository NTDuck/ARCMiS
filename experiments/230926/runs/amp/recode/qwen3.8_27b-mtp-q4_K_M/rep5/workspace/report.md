# Validation Report — AMP C → Rust translation

## Build
- `cargo build`: **success** (no errors, no warnings).

## Tests (`cargo test`)
- Unit tests (src/lib.rs): **5 passed, 0 failed**
  - `empty_argv` — ok
  - `single_arg` — ok
  - `binary_arg_with_nul` — ok
  - `truncated_buffer_errors` — ok
  - `too_many_args_rejected` (should_panic) — ok
- Integration tests (tests/test.rs): **1 passed, 0 failed**
  - `roundtrip` — ok
- Doc-tests: 0 (none defined).

**Total: 6 passed, 0 failed.**

## Failing tests
None.

## Coverage vs. plan function list

| Plan symbol | Target symbol | Test coverage |
|---|---|---|
| `AMP_VERSION` | `VERSION` | exercised in `empty_argv`, `single_arg`, `roundtrip` |
| `amp_t` | `AmpMessage` | exercised in all tests |
| `amp_encode` | `encode` | `empty_argv`, `single_arg`, `binary_arg_with_nul`, `too_many_args_rejected`, `roundtrip` |
| `amp_decode` | `decode` | `empty_argv`, `single_arg`, `truncated_buffer_errors`, `roundtrip` |
| `amp_decode_arg` | `decode_arg` | `empty_argv`, `single_arg`, `binary_arg_with_nul`, `truncated_buffer_errors`, `roundtrip` |
| `main` (tests/test.c) | `roundtrip` (tests/test.rs) | present and passing |
| `read_u32_be` / `write_u32_be` | inlined (`u32::from_be_bytes` / `to_be_bytes`) | covered indirectly via encode/decode tests |
| `AmpError` (new type) | `AmpError::{Truncated, BadLength, TooManyArgs}` | `Truncated` and `BadLength` asserted in `truncated_buffer_errors`; `TooManyArgs` variant exists (panic path covered by `too_many_args_rejected`) |

**Uncovered functions: none.**

## Conclusion
The translation matches the plan's name mapping and behavior; build and full test suite pass.
