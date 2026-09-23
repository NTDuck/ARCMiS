# Validation Report — ulidgen (C → Rust)

## Build
- `cargo build`: **OK** (lib + bin compile, no errors).

## Tests (`cargo test`)
- `tests/ulid.rs`: **4 passed, 0 failed**
  - `ulid_length` ... ok
  - `ulid_structure` ... ok
  - `ulid_uniqueness` ... ok
  - `ulid_sortability` ... ok
- Unit tests (lib/bin): 0 (none defined)
- Doc-tests: 0

**Result: all_success = true** (build clean, all tests pass).

## CLI smoke (manual)
- `cargo run -- -n 3` → 3 ULIDs, same-ms increment visible (`...Y97Y` → `...Y97Z` → `...Y980`), exit 0.
- `echo hi | cargo run -- -t` → `<ULID> hi`, exit 0.

## Function coverage vs. plan
| Plan symbol (C) | Rust | Covered by test? |
|---|---|---|
| `ulidgen_r` | `UlidGen::next` | Yes (all 4 tests) |
| `b32alphabet` | `B32` (const) | n/a (constant, used by tests) |
| `main` | `main` (CLI) | **No** (manual smoke only) |
| `is_valid_ulid` | `is_valid_ulid` | Yes (helper used in tests) |
| `test_ulid_length` | `ulid_length` | Yes |
| `test_ulid_structure` | `ulid_structure` | Yes |
| `test_ulid_uniqueness` | `ulid_uniqueness` | Yes |
| `test_ulid_sortability` | `ulid_sortability` | Yes |

### Uncovered functions
- `main` (CLI entry point) — exercised only by manual smoke test, not by `cargo test`.

## Failures
- None.
