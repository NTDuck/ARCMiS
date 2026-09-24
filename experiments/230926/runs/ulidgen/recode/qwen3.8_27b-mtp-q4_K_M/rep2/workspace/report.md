# Validation report — ulidgen (C → Rust)

## Build & test

- Command: `cargo test` (from workspace root)
- Result: **success** — 12 tests passed, 0 failed, 0 ignored.
  - `src/lib.rs` unit tests: 4 passed (`test_ulid_length`, `test_ulid_structure`, `test_ulid_uniqueness`, `test_ulid_sortability`)
  - `tests/ulid.rs` integration: 4 passed (same four tests, driving `ulidgen_r`/`ulid`/`B32_ALPHABET`)
  - `tests/cli.rs` integration: 4 passed (`test_cli_default_one_ulid`, `test_cli_n3`, `test_cli_n0`, `test_cli_tag`)
- No failing tests; no diagnostics to record.

## Function coverage vs. plan

| Plan symbol | Coverage |
|---|---|
| `ulidgen_r` | covered — `tests/ulid.rs::test_ulid_length` calls it directly; exercised by all ULID tests and CLI tests |
| `B32_ALPHABET` | covered — used by `is_valid_ulid` in both integration test files |
| `ulid()` | covered — all ULID tests |
| `main` (CLI, `-n`/`-t`) | covered — all 4 `tests/cli.rs` tests (default, `-n 3`, `-n 0`, `-t` stdin tagging) |
| `is_valid_ulid` | covered — helper used by `tests/ulid.rs` and `tests/cli.rs` |
| `test_ulid_length` / `test_ulid_structure` / `test_ulid_uniqueness` / `test_ulid_sortability` | all present and passing (unit + integration) |

Uncovered functions: **none**.

## Conclusion

`all_success = true`. The translated codebase builds cleanly and the full test suite is green; every function in the plan's inventory has test coverage. No test generation was required (`test_generation: false`).
