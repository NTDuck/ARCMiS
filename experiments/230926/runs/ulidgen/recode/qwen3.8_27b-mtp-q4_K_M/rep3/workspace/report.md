# Validation Report — ulidgen (C → Rust)

Date: validation run by pipeline validator
Test command: `cargo test`

## Build

- `cargo build`: **OK** (no errors, no warnings surfaced).

## Test Results

`cargo test` — **12 passed, 0 failed**:

| Suite | Tests | Result |
|---|---|---|
| `tests/cli.rs` | `default_prints_one_ulid`, `n_mode_prints_n_ulids`, `n_zero_prints_nothing`, `invalid_n_fails_with_parse_long_error`, `t_mode_prefixes_each_stdin_line`, `t_mode_empty_stdin_prints_nothing` | 6/6 ok |
| `tests/ulid.rs` | `ulid_length`, `ulid_structure`, `ulid_uniqueness`, `ulid_sortability` | 4/4 ok |
| `tests/ulid_fn.rs` | `ulid_returns_26_valid_chars`, `ulid_is_unique_across_calls` | 2/2 ok |
| Doc-tests | — | 0 (none) |

No failing tests; no diagnostics to record.

## Smoke Checks (plan §Verification)

- `cargo run -- -n 3` → printed 3 ULIDs, exit 0. Output shows the same-millisecond increment path working (`...W1KF`, `...W1KG`, `...W1KH`).
- `printf 'a\nb\n' | cargo run -- -t` → each stdin line prefixed with a ULID, exit 0.

## Function Coverage vs. Plan

| Plan function | Coverage |
|---|---|
| `ulidgen_r` (src/lib.rs) | ✅ `tests/ulid.rs` (length/structure/uniqueness/sortability via shared buffer), `tests/ulid_fn.rs` |
| `B32_ALPHABET` (src/lib.rs) | ✅ exercised via `is_valid_ulid` in all three test suites |
| `ulid()` (src/lib.rs) | ✅ `tests/ulid_fn.rs` |
| `parse_long` (src/main.rs) | ✅ `invalid_n_fails_with_parse_long_error` |
| `Args` / `main` (src/main.rs) | ✅ `tests/cli.rs` (default, `-n`, `-t`, error path) |
| `is_valid_ulid` (test helper) | ✅ used by all suites |

**Uncovered functions: none.**

## Conclusion

Build succeeds, all 12 tests pass, smoke checks match the C semantics described in the plan, and every planned function has test coverage. The translator's report (12/12 green, no uncovered functions) is confirmed.
