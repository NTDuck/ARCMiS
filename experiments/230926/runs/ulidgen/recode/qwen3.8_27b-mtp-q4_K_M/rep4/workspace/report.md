# Validation Report — ulidgen (C → Rust)

## Build
- `cargo build`: **success** (no errors, no warnings).

## Test run (`cargo test`)
- `src/lib.rs` unit tests (port of `tests/test.c`): **4 passed, 0 failed**
  - `tests::test_ulid_length` ... ok
  - `tests::test_ulid_structure` ... ok
  - `tests::test_ulid_uniqueness` ... ok
  - `tests::test_ulid_sortability` ... ok
- `src/main.rs` unit tests: 0 tests (none defined)
- `tests/cli.rs` integration tests (newly generated for `main`): **9 passed, 0 failed**
  - `cli_default_prints_one_valid_ulid` ... ok
  - `cli_n_prints_n_sorted_unique_ulids` ... ok
  - `cli_n_zero_prints_nothing_and_succeeds` ... ok
  - `cli_tag_mode_prefixes_every_stdin_line` ... ok
  - `cli_tag_mode_empty_input_succeeds_silently` ... ok
  - `cli_non_numeric_n_exits_1_with_usage` ... ok
  - `cli_bare_n_exits_1_with_usage` ... ok
  - `cli_unknown_flag_exits_1_with_usage` ... ok
  - `cli_write_failure_exits_1` ... ok
- Doc-tests: 0 tests
- **Total: 13 passed, 0 failed.** Suite re-run a second time: identical result (stable).

## Test generation (for previously uncovered `main`)
`tests/cli.rs` was added (integration tests spawning the built binary via
`CARGO_BIN_EXE_ulidgen`). Each test mirrors the C `src/ulidgen.c` `main`
behavior:
- default (no args) → exactly one valid 26-char Crockford-base32 ULID, exit 0
- `-n 5` → 5 valid ULIDs, strictly increasing (stateful same-ms increment
  path), exit 0
- `-n 0` → no output, exit 0
- `-t` with `alpha\nbeta\n` on stdin → each line prefixed with a valid ULID
  + space, original text preserved, exit 0
- `-t` with empty stdin → no output, exit 0
- `-n abc`, bare `-n`, `-x` → usage on stderr, exit 1
- write failure (stdout read end closed, 100 000 lines) → EPIPE → exit 1
  (mirrors C `exit(!!ferror(stdout))`; Rust ignores SIGPIPE so the write
  returns Err)

No existing tests were modified or weakened.

## Function coverage vs plan
| Plan function | Test coverage |
|---|---|
| `ulidgen_r` | ✅ all 4 unit tests + `cli_n_prints_n_sorted_unique_ulids` (increment path) |
| `B32` | ✅ used by `is_valid_ulid` and all tests |
| `is_valid_ulid` | ✅ used by `test_ulid_structure` |
| `test_ulid_length` / `test_ulid_structure` / `test_ulid_uniqueness` / `test_ulid_sortability` | ✅ all executed and passing |
| `main` (CLI) | ✅ now covered by 9 integration tests in `tests/cli.rs` |

## Failures
None.

## Conclusion
Build clean; 13/13 tests pass (4 pre-existing + 9 generated for `main`);
every function in the plan now has test coverage.
