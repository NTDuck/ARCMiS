# ulidgen — C → Rust Translation Plan

Test command: `cargo test`

## 1. Fragment extraction (source symbols)

| Source file | Symbol | Kind | Target location |
|---|---|---|---|
| `src/ulid.h` | `ulidgen_r` | declaration | `src/ulid.rs` |
| `src/ulid.c` | `ulidgen_r` | function | `src/ulid.rs::ulidgen_r` |
| `src/ulid.c` | `b32alphabet` | static const | `src/ulid.rs::B32_ALPHABET` |
| `src/ulidgen.c` | `main` | function | `src/main.rs::main` |
| `tests/test.c` | `is_valid_ulid` | helper fn | `tests/test.rs::is_valid_ulid` |
| `tests/test.c` | `test_ulid_length` | test | `tests/test.rs::test_ulid_length` |
| `tests/test.c` | `test_ulid_structure` | test (commented out in C main; enable it) | `tests/test.rs::test_ulid_structure` |
| `tests/test.c` | `test_ulid_uniqueness` | test | `tests/test.rs::test_ulid_uniqueness` |
| `tests/test.c` | `test_ulid_sortability` | test | `tests/test.rs::test_ulid_sortability` |
| `tests/test.c` | `main` (test runner) | function | dropped — replaced by the `cargo test` harness |

## 2. Name mapping (C → Rust)

| C name | Rust name | Reason |
|---|---|---|
| `ulidgen_r` | `ulidgen_r` | preserved (valid Rust identifier) |
| `b32alphabet` | `B32_ALPHABET` | Rust `const` naming convention (SCREAMING_SNAKE) |
| `main` | `main` | preserved |
| `is_valid_ulid`, `test_ulid_*` | unchanged | preserved; `#[test]` replaces the C `main` runner |
| `getentropy(2)` | `getrandom::getrandom` | crate is the direct syscall counterpart |
| `clock_gettime(CLOCK_REALTIME)` | `SystemTime::now()` | std |
| `nanosleep` | `std::thread::sleep` | std |
| `getopt("n:t")` | `getopts::Options` | crate is the direct POSIX-getopt counterpart |
| `getdelim` | `BufRead::read_line` | std |
| `setvbuf(_IOLBF)` | `std::io::LineWriter` | std |
| `atol` | `str::parse::<i64>()` | std |
| `abort()` | `std::process::abort()` | std |
| `exit(!!ferror(stdout))` | `std::process::exit(1)` on write error | std |

## 3. Skeleton (already written, compiles)

- `src/lib.rs` — `pub mod ulid;`
- `src/ulid.rs` — `B32_ALPHABET` const + `pub fn ulidgen_r(ulid: &mut [u8; 27])` stub with porting notes
- `src/main.rs` — `main()` stub with porting notes
- `tests/test.rs` — `is_valid_ulid` + 4 `#[test]` stubs with porting notes

## 4. Part A — source files, bottom-up dependency order

### A1. `src/lib.rs`
- Already final: `pub mod ulid;` plus crate doc comment. No changes needed.

### A2. `src/ulid.rs` (depends on: `getrandom`, `std::time`)
Port `src/ulid.c::ulidgen_r` into `pub fn ulidgen_r(ulid: &mut [u8; 27])`:
1. `ulid[26] = 0` (NUL, as in C).
2. `t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64`.
3. Timestamp digits: `let mut same = true;` then `for i in (0..10).rev() { let d = B32_ALPHABET[(t % 32) as usize]; if ulid[i] != d { ulid[i] = d; same = false; } t /= 32; }` — `same` starts `true` and is cleared on any *mismatch* (C semantics).
4. If `same` (same-ms repeat, buffer reused by caller):
   - Walk `buf = &mut ulid[10..]` from index 15 down; while `buf[i] == b'Z'` set `buf[i] = b'0'`.
   - If all 16 were `'Z'` (index underflows): `thread::sleep(Duration::from_nanos(1_234_567))` then **recurse** `ulidgen_r(ulid)` and return.
   - Else if `buf[i]` is found in `B32_ALPHABET`: set `buf[i]` to the next alphabet char and return.
   - Else (invalid char in random part): fall through to re-randomize.
5. Else: `let mut rnd = [0u8; 16]; getrandom::getrandom(&mut rnd).unwrap_or_else(|_| std::process::abort());` then `for i in 0..16 { buf[i] = B32_ALPHABET[rnd[i] as usize % 32]; }` — **keep the deliberate `% 32` modulo bias**, do not add rejection sampling.

### A3. `src/main.rs` (depends on: A2, `getopts`, `std::io`)
Port `src/ulidgen.c::main`:
1. `getopts::Options`: `optopt("n", "N", "Print N consecutive ULID", "N")`, `optflag("t", "", "prefix stdin lines with a ULID")`; parse; `n` default 1 via `parse::<i64>()` (C `atol`), `tflag` bool.
2. One reused buffer: `let mut ulid = [0u8; 27];` (critical for same-ms uniqueness).
3. `-t` mode: `let stdout = std::io::stdout(); let mut out = LineWriter::new(stdout.lock());` loop `stdin.lock().read_line(&mut line)`; `out.write_all(ulid_str.as_bytes())?; out.write_all(b" ")?; out.write_all(line.as_bytes())?;` — line keeps its trailing `\n` (C `getdelim` behavior).
4. Default mode: `for _ in 0..n { ulidgen_r(&mut ulid); out.write_all(ulid_str)?; out.write_all(b"\n")?; }`
5. Convert safely: `let ulid_str = std::str::from_utf8(&ulid[..26]).unwrap();` (buffer only ever holds alphabet chars + NUL).
6. On any write error: `std::process::exit(1)` (mirrors `exit(!!ferror(stdout))`); success exits 0.

## 5. Part B — test files, bottom-up dependency order

### B1. `tests/test.rs` (depends on: A2 via `ulidgen::ulid::ulidgen_r`)
Port `tests/test.c`, enabling the structure test that was commented out in C `main`:
- `is_valid_ulid(ulid: &str) -> bool`: `len() == 26` and every char in `B32_ALPHABET`.
- `test_ulid_length`: `let mut u = [0u8; 27]; ulidgen_r(&mut u); assert_eq!(std::str::from_utf8(&u[..26]).unwrap().len(), 26);`
- `test_ulid_structure`: same setup, `assert!(is_valid_ulid(...))`.
- `test_ulid_uniqueness`: two consecutive calls on the **same reused buffer** must differ.
- `test_ulid_sortability`: generate u1, `thread::sleep(Duration::from_micros(1_500))` (C 1.5 ms), generate u2, assert `u1 < u2` lexicographically.

## 6. Verification
- `cargo build` — must compile without errors.
- `cargo test` — all 4 tests pass.
- Smoke: `cargo run -- -n 3` prints 3 distinct, sorted ULIDs; `echo hi | cargo run -- -t` prints `<ULID> hi`.
