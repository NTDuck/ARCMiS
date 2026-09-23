# ulidgen — C → Rust Translation Plan

Target: Rust. Test command: `cargo test`.
Design: see `design.md`. Skeleton files (compilable stubs) are already in place:
`Cargo.toml`, `src/lib.rs`, `src/main.rs`, `tests/ulid.rs`.

## 1. Fragment extraction (source symbols)

| Source file | Symbol | Kind | Role |
|---|---|---|---|
| `src/ulid.c` | `ulidgen_r(char ulid[27])` | function | Core: fill 26-char ULID (10 ts + 16 random), stateful via buffer reuse |
| `src/ulid.c` | `b32alphabet` | static const | Crockford-Base32 alphabet |
| `src/ulid.h` | `ulidgen_r` prototype | decl | Public API |
| `src/ulidgen.c` | `main(int argc, char *argv[])` | function | CLI: `-n N`, `-t`, exit status |
| `tests/test.c` | `is_valid_ulid(const char*)` | function | Test helper |
| `tests/test.c` | `test_ulid_length` | function | Test: length 26 |
| `tests/test.c` | `test_ulid_structure` | function | Test: valid alphabet chars |
| `tests/test.c` | `test_ulid_uniqueness` | function | Test: consecutive ULIDs differ |
| `tests/test.c` | `test_ulid_sortability` | function | Test: lexicographic order across ms |

## 2. Name mapping (C → Rust)

| C symbol | Rust symbol | Reason for change |
|---|---|---|
| `ulidgen_r(char[27])` | `UlidGen::next(&mut self) -> String` | C statefulness comes from caller buffer reuse; Rust models it as a struct `UlidGen` with field `last: [u8; 26]` and a `next()` method. Name `ulidgen_r` not preserved (Rust convention: type + method). |
| `b32alphabet` | `B32` | Rust const naming convention; same value. |
| `main` | `main` | Preserved. |
| `is_valid_ulid` | `is_valid_ulid` | Preserved (test helper). |
| `test_ulid_length` | `ulid_length` | `#[test]` fn; `test_` prefix redundant in Rust. |
| `test_ulid_structure` | `ulid_structure` | Same. |
| `test_ulid_uniqueness` | `ulid_uniqueness` | Same. |
| `test_ulid_sortability` | `ulid_sortability` | Same. |
| `getentropy(2)` | `getrandom::getrandom` | Portable crate replacement. |
| `clock_gettime(CLOCK_REALTIME)` | `SystemTime::now().duration_since(UNIX_EPOCH)` | std replacement. |
| `nanosleep` | `std::thread::sleep(Duration::from_nanos(1_234_567))` | std replacement. |
| `getopt` | manual `std::env::args` loop | Minimal deps (clap optional, not used). |
| `atol` | `parse::<i64>()` with fallback to 1 | Rust parsing is strict; keep lenient behavior. |
| `getdelim` | `stdin.lock().lines()` | std `BufRead`. |
| `setvbuf(_IOLBF)` | locked stdout + `writeln!` | Equivalent line-oriented output. |
| `fflush` / `ferror` | `out.flush().is_ok()` → exit 0/1 | Preserve exit-status semantics. |
| `abort()` | `std::process::abort()` | Preserved on entropy failure. |

## 3. Part A — source files, bottom-up dependency order

### A1. `Cargo.toml` (already written)
- Package `ulidgen`, edition 2021, empty `[workspace]` (isolation),
  dependency `getrandom = "0.2"`, `[[bin]]` pointing at `src/main.rs`.
- Verify: `cargo build` resolves.

### A2. `src/lib.rs` (skeleton exists — fill in `UlidGen::next`)
Depends on: `getrandom`, `std::time`, `std::thread`.
Implement exactly per design.md §2/§4:
1. `ulid = self.last` (buffer reuse); `same = true`.
2. `t = SystemTime::now().duration_since(UNIX_EPOCH).as_millis() as u64`
   (unwrap_or(0) on error).
3. For `i in (0..10).rev()`: `c = B32[(t % 32) as usize]`;
   if `ulid[i] != c` set it and `same = false`; `t /= 32`.
4. If `same`: increment `ulid[10..26]` right-to-left — zero trailing `Z`s;
   if index drops below 10 (all `Z`), `sleep(1_234_567 ns)` and recurse
   `self.next()`; else find `pos` of `ulid[i]` in `B32`, require
   `pos + 1 < 32`, set `ulid[i] = B32[pos + 1]`, store `self.last`, return.
   If char not in alphabet, fall through to re-randomize.
5. Else: `getrandom(&mut rnd[16])` (`abort()` on error);
   `ulid[10 + i] = B32[(rnd[i] % 32) as usize]` for `i in 0..16`.
6. `self.last = ulid`; return `String::from_utf8(ulid.to_vec()).unwrap()`.
- Verify: `cargo build` (no warnings ideally).

### A3. `src/main.rs` (skeleton exists — fill in CLI)
Depends on: `ulidgen` lib (A2), `std::io`.
1. Parse `std::env::args().skip(1)`:
   `"-n"` → next arg `parse::<i64>()` (fallback 1, C `atol` leniency);
   `"-t"` → `tflag = true`; unknown → `eprintln!` + `exit(2)`.
2. `tflag` branch: for each `Ok(line)` in `stdin.lock().lines()`,
   `writeln!(out, "{} {}", gen.next(), line)`; break on read error.
3. Else: `for _ in 0..n.max(0) { writeln!(out, "{}", gen.next()); }`.
4. Keep existing tail: `flush()` → `exit(0/1)` (C `fflush`/`ferror`).
- Verify: `cargo build`; manual: `cargo run -- -n 3`,
  `echo hi | cargo run -- -t`.

## 4. Part B — test files, bottom-up dependency order

### B1. `tests/ulid.rs` (already written — verify, do not change API)
Depends on: `ulidgen` lib (A2).
- `ulid_length`, `ulid_structure`, `ulid_uniqueness`, `ulid_sortability`
  mirror `tests/test.c` (structure test was commented out in C `main`;
  here all 4 run under `cargo test`).
- `is_valid_ulid` helper preserved.
- Optional (design.md §5.9): add a same-ms increment test by seeding
  `UlidGen.last` (e.g. expose a `#[cfg(test)]` setter or `pub(crate)`
  accessor) to exercise the increment branch deterministically.
- Verify: `cargo test` — all 4 tests pass.

## 5. Final verification
- `cargo build` — lib + bin compile.
- `cargo test` — 4 mirrored tests pass.
- Manual smoke: `cargo run -- -n 3` prints 3 ULIDs;
  `echo hi | cargo run -- -t` prints `<ULID> hi`;
  exit code 0 on success, 1 on stdout failure.
