# ulidgen — C → Rust Translation Plan

Workspace: Cargo package `ulidgen` (edition 2021), lib + bin + integration tests.
Only external dependency: `getrandom` v0.2 (`fill` API, replaces `getentropy(2)`).
Everything else (time, sleep, I/O, args, tests) comes from `std`.
Test command: `cargo test`.

## 1. Fragment extraction (source symbols)

| Source file | Symbol | Kind | Notes |
|---|---|---|---|
| `src/ulid.h` | `ulidgen_r` | declaration | `void ulidgen_r(char[27])` |
| `src/ulid.c` | `ulidgen_r` | function | core ULID generator; static `b32alphabet`; same-ms in-place increment, all-'Z' wrap → 1.23 ms sleep + recursion; `getentropy` random fill; `abort()` on entropy failure |
| `src/ulidgen.c` | `main` | function | CLI: `getopt("n:t")`, `-n N` (atol, default 1), `-t` stdin tagging via `getdelim`, `setvbuf` line-buffering, `exit(!!ferror(stdout))` |
| `tests/test.c` | `is_valid_ulid` | helper fn | length 26 + alphabet membership |
| `tests/test.c` | `test_ulid_length` | test | `strlen == 26` |
| `tests/test.c` | `test_ulid_structure` | test | alphabet validity (commented out in C `main`, but ported) |
| `tests/test.c` | `test_ulid_uniqueness` | test | two consecutive ULIDs differ |
| `tests/test.c` | `test_ulid_sortability` | test | ULID1 < ULID2 after 1.5 ms `nanosleep` |
| `tests/test.c` | `main` | test runner | replaced by Rust's `#[test]` harness |

## 2. Name mapping (C → Rust)

| C name | Rust name | Reason |
|---|---|---|
| `ulidgen_r` | `ulidgen_r` | preserved; signature becomes `pub fn ulidgen_r(ulid: &mut [u8; 27])` to mirror the caller-buffer contract |
| `b32alphabet` (static in `ulid.c`) | `B32_ALPHABET` | Rust `const` naming convention; promoted to `pub const &[u8; 32]` |
| `main` (CLI, `src/ulidgen.c`) | `main` | preserved; `fn main() -> io::Result<()>` for nonzero exit on write error |
| `is_valid_ulid` | `is_valid_ulid` | preserved (test helper) |
| `test_ulid_length` | `ulid_length` | Rust `#[test]` names drop the redundant `test_` prefix |
| `test_ulid_structure` | `ulid_structure` | same |
| `test_ulid_uniqueness` | `ulid_uniqueness` | same |
| `test_ulid_sortability` | `ulid_sortability` | same |
| `main` (test runner, `tests/test.c`) | — (removed) | Rust test harness replaces the manual runner |
| — (new) | `ulid()` | added convenience `pub fn ulid() -> String` per design (zeroed buffer → first call randomizes, matching C's `char ulid[27] = {0}`) |

## 3. Skeleton status

Skeleton files already exist in the workspace and compile (`cargo build` and
`cargo test --no-run` both pass):

- `Cargo.toml` / `Cargo.lock` — package `ulidgen`, dep `getrandom = "0.2"`.
- `src/lib.rs` — `B32_ALPHABET`, `ulidgen_r` (stub, `todo!`), `ulid()` (stub),
  `#[cfg(test)] mod tests` with 5 stub tests.
- `src/main.rs` — `main` stub (`todo!`).
- `tests/ulid.rs` — `is_valid_ulid` helper + 4 stub `#[test]` fns.

Implementers must replace each `todo!`/`TODO` body per design.md §3 and keep the
public signatures unchanged.

## 4. Implementation plan

### Part A — source files (bottom-up dependency order)

1. **`src/lib.rs`** (port of `src/ulid.c` + `src/ulid.h`)
   - Implement `ulidgen_r(&mut [u8; 27])` mirroring the C logic 1:1:
     - ms timestamp via `SystemTime::now().duration_since(UNIX_EPOCH)` →
       `secs*1000 + nanos/1_000_000` as `u64`;
     - encode into `ulid[0..10]` (loop `i` 9..=0, `t /= 32`), tracking `same`;
     - same-ms branch: scan `ulid[10..26]` from index 15 down, wrap `'Z'`→`'0'`;
       all wrapped → `thread::sleep(Duration::from_nanos(1_234_567))` + recurse;
       else advance char via linear alphabet lookup (like `strchr`); invalid
       char → fall through to randomize;
     - random branch: `getrandom::fill(&mut [0u8; 16])` (`.expect(...)` ≈ C
       `abort()`), encode `B32_ALPHABET[rnd[i] % 32]` into `ulid[10..26]`;
     - keep `ulid[26] == 0` sentinel.
   - Implement `ulid() -> String` on top of `ulidgen_r` (zeroed buffer).
   - Fill in the 5 unit tests in `mod tests` (alphabet length, length, structure,
     uniqueness, sortability with 2 ms sleep).
   - Depends on: `getrandom`, `std` only.

2. **`src/main.rs`** (port of `src/ulidgen.c`)
   - Manual `std::env::args().skip(1)` parsing: `-n N` (`parse::<i64>()
     .unwrap_or(1)`, atol-like leniency), `-t` flag; unknown flag → usage to
     stderr, exit 1.
   - `-t` mode: `stdin.lock().lines()` loop, `ulidgen_r(&mut buf)`,
     `write!(stdout, "{} {}\n", ulid_str, line)` (re-append `\n` since
     `lines()` strips it — C `getdelim` kept it).
   - `-n` mode: loop `0..n`, `println!`.
   - `fn main() -> io::Result<()>` + explicit `flush()` for parity with
     `exit(!!ferror(stdout))`.
   - Depends on: `src/lib.rs` (`ulidgen_r`).

### Part B — test files (bottom-up dependency order)

1. **`tests/ulid.rs`** (port of `tests/test.c`)
   - `is_valid_ulid` helper (length 26 + Crockford alphabet membership).
   - `ulid_length`, `ulid_structure` (prints generated ULID, like C),
     `ulid_uniqueness`, `ulid_sortability` (2 ms sleep vs C's 1.5 ms).
   - Depends on: `src/lib.rs` public API (`ulid`).

### Verification

- `cargo build` — lib + bin compile.
- `cargo test` — 5 unit tests (lib.rs) + 4 integration tests (tests/ulid.rs).
- Smoke: `cargo run -- -n 3` → 3 distinct 26-char ULIDs;
  `echo hello | cargo run -- -t` → `<ULID> hello`.
