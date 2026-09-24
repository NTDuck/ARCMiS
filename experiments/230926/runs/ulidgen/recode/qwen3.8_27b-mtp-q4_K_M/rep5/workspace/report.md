# Validation Report — ulidgen (C → Rust)

Validator run: `cargo build` + `cargo test` (workspace root).

## Build

- `cargo build`: **OK** — clean, no warnings, binary `ulidgen` produced.

## Test results

`cargo test`: **11 passed, 0 failed** (0 ignored, 0 doc-tests).

| Test target | Tests | Result |
|---|---|---|
| `tests/test.rs` (port of `tests/test.c`) | `ulid_length`, `ulid_structure`, `ulid_uniqueness`, `ulid_sortability` | 4 passed |
| `tests/cli.rs` (port of `src/ulidgen.c` behavior) | `cli_default_prints_one_ulid`, `cli_n_generates_n_unique_ulids`, `cli_n_zero_prints_nothing`, `cli_t_tags_each_line`, `cli_t_preserves_missing_trailing_newline`, `cli_t_empty_input_prints_nothing`, `cli_unknown_flag_exits_1_with_usage` | 7 passed |

### Failing tests

None. No diagnostics to collect.

## Coverage vs. plan function list

| Plan function (C → Rust) | Test coverage |
|---|---|
| `ulidgen_r` (`src/ulid.c` → `src/lib.rs`) | `ulid_length`, `ulid_structure`, `ulid_uniqueness`, `ulid_sortability` + all CLI tests (same-millisecond increment path exercised by `cli_n_generates_n_unique_ulids`) |
| `main` (`src/ulidgen.c` → `src/main.rs`) | all 7 `tests/cli.rs` tests (both `-n` and `-t` modes, default, zero, error path) |
| `is_valid_ulid` (test helper) | used by `ulid_structure` and CLI tests |
| `test_ulid_length` → `ulid_length` | present, passing |
| `test_ulid_structure` → `ulid_structure` | present, passing (note: C `main` never called it; Rust runs it — strictly more coverage) |
| `test_ulid_uniqueness` → `ulid_uniqueness` | present, passing |
| `test_ulid_sortability` → `ulid_sortability` | present, passing |

**Uncovered functions: none.**

## Notes

- Name mapping from the plan is preserved (`ulidgen_r`, `B32_ALPHABET`, `is_valid_ulid`, `test_` prefix dropped per cargo convention).
- `test_generation` was `false`; no additional tests were written.
- No translated source was modified by the validator.

## Verdict

**all_success = true** — build clean, 11/11 tests pass, every planned function covered.
