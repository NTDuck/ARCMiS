# Validation Report — ulidgen (C → Rust)

## Build
- `cargo build`: **success** (clean, no errors).

## Tests (`cargo test`)
- **17 passed, 0 failed** (4 lib unit tests + 9 CLI integration tests in `tests/cli.rs` + 4 integration tests in `tests/ulid.rs`).
- No failing tests; no diagnostics to collect.

| Test | Result |
|---|---|
| `tests::test_ulid_length` (lib) | ok |
| `tests::test_ulid_structure` (lib) | ok |
| `tests::test_ulid_uniqueness` (lib) | ok |
| `tests::test_ulid_sortability` (lib) | ok |
| `cli_n_generates_n_ulids` | ok |
| `cli_default_single_ulid` | ok |
| `cli_t_tags_stdin_lines` | ok |
| `cli_bad_n_operand_exits_2` | ok |
| `cli_missing_n_operand_exits_2` | ok |
| `cli_unknown_option_exits_2` | ok |
| `cli_n_zero_prints_nothing` (added) | ok |
| `cli_t_empty_stdin` (added) | ok |
| `cli_t_preserves_spaces_in_line` (added) | ok |
| `test_ulid_length` (integration) | ok |
| `test_ulid_structure` (integration) | ok |
| `test_ulid_uniqueness` (integration) | ok |
| `test_ulid_sortability` (integration) | ok |

## Test generation
The message flagged `main` as uncovered, but `tests/cli.rs` already exercised the
CLI (arg parsing, `-n`, `-t`, exit codes). Per `test_generation: true`, three
additional tests for `main` were appended to `tests/cli.rs` (no existing test
modified or weakened):
- `cli_n_zero_prints_nothing` — `-n 0` prints no lines, exit 0 (C `atol("0")` → zero iterations).
- `cli_t_empty_stdin` — `-t` with empty stdin prints nothing, exit 0.
- `cli_t_preserves_spaces_in_line` — `-t` preserves lines containing spaces verbatim after `ULID `.

Full suite re-run after additions: **17 passed, 0 failed**.

## Plan function coverage
| Plan function | Coverage |
|---|---|
| `ulidgen` (src/lib.rs) | covered (length/structure/uniqueness/sortability, lib + integration) |
| `ulidgen_fresh` (src/lib.rs) | covered (lib unit tests) |
| `is_valid_ulid` (helper) | covered (used by structure tests) |
| `main` (src/main.rs, CLI) | covered (9 tests in `tests/cli.rs`, incl. 3 added this run) |

## Conclusion
Build succeeds, all 17 tests pass, every plan function has test coverage, and
CLI behavior matches the plan's checklist (26-char Crockford Base32 output,
`-t` preserves lines verbatim, exit 0 on success, exit 2 on bad args).
