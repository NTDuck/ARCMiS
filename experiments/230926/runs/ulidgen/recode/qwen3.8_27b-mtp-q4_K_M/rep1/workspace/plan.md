# ulidgen — C → Rust Implementation Plan

Translate the public-domain C utility `ulidgen` (ULID generator / stdin tagger)
into a zero-dependency Rust Cargo package. Test command: `cargo test`.

## Fragment inventory (source → target)

| Source fragment | Location | Target | Target location |
|---|---|---|---|
| `ulidgen_r` (decl) | `src/ulid.h` | `pub fn ulidgen_r(ulid: &mut [u8; 27])` | `src/lib.rs` |
| `ulidgen_r` (impl) | `src/ulid.c` | `pub fn ulidgen_r(ulid: &mut [u8; 27])` | `src/lib.rs` |
| `b32alphabet` (local const) | `src/ulid.c` | `pub const B32_ALPHABET: &str` | `src/lib.rs` |
| `main` (CLI) | `src/ulidgen.c` | `fn main()` | `src/main.rs` |
| `is_valid_ulid` (helper) | `tests/test.c` | `fn is_valid_ulid(ulid: &str) -> bool` | `tests/test.rs` |
| `test_ulid_length` | `tests/test.c` | `#[test] fn test_ulid_length()` | `tests/test.rs` |
| `test_ulid_structure` | `tests/test.c` | `#[test] fn test_ulid_structure()` | `tests/test.rs` |
| `test_ulid_uniqueness` | `tests/test.c` | `#[test] fn test_ulid_uniqueness()` | `tests/test.rs` |
| `test_ulid_sortability` | `tests/test.c` | `#[test] fn test_ulid_sortability()` | `tests/test.rs` |
| (new, per design) | — | `pub fn ulid() -> String` wrapper | `src/lib.rs` |

## Name mapping

- `ulidgen_r` → `ulidgen_r` (name preserved; C `char[27]` becomes `&mut [u8; 27]`).
- `main` → `main` (preserved).
- `is_valid_ulid` → `is_valid_ulid` (preserved; `const char *` → `&str`).
- `test_ulid_*` → same names as `#[test]` functions.
- `b32alphabet` → `B32_ALPHABET` (Rust const naming convention; made `pub` so
  tests can reference it).
- C library calls → std: `clock_gettime(CLOCK_REALTIME)` →
  `SystemTime::now().duration_since(UNIX_EPOCH)`; `getentropy` →
  `std::os::unix::fs::getentropy`; `nanosleep` → `std::thread::sleep`;
  `getopt` → manual `std::env::args()` parsing; `setvbuf(_IOLBF)` → nothing
  (Rust stdout is line-buffered); `getdelim` → `BufRead::read_line`;
  `atol` → `str::parse::<i64>()`; `abort()` → `std::process::abort()`;
  `exit(!!ferror(stdout))` → flush/write error → `std::process::exit(1)`.
- Locals `n`, `tflag` preserved as `n: i64`, `tflag: bool`.

## Part A — source files (bottom-up dependency order)

### A1. `Cargo.toml` (already in place)
- Package `ulidgen`, edition 2021, empty `[dependencies]`, explicit `[lib]`
  and `[[bin]]` sections, empty `[workspace]` table to stay standalone.
- Verify: `cargo check` passes.

### A2. `src/lib.rs` — core generator (depends only on std)
Fill in the two stubs, porting `src/ulid.c` line-for-line in semantics:

1. `pub fn ulidgen_r(ulid: &mut [u8; 27])`:
   - `ulid[26] = 0` (NUL terminator, as in C).
   - Timestamp ms: `let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();`
     `let mut t = d.as_secs() as u64 * 1000 + d.subsec_nanos() as u64 / 1_000_000;`
   - Encode loop `for i in (0..10).rev()`: if `ulid[i] != B32_ALPHABET.as_bytes()[t % 32]`
     set it and clear `same`; else keep `same = true`. (C: `for (i = 9; i >= 0; i--, t /= 32)`.)
   - If `same` (same millisecond as previous call): scan `i` from 15 down while
     `ulid[i] == b'Z'` → set `b'0'`; if `i < 0`: `thread::sleep(Duration::from_nanos(1_234_567))`
     and recurse `ulidgen_r(ulid)` then return; else if the byte is in
     `B32_ALPHABET`, advance to the next alphabet byte and return; otherwise
     fall through to re-randomization.
   - Random fill: `let mut rnd = [0u8; 16];`
     `std::os::unix::fs::getentropy(&mut rnd).unwrap_or_else(|_| std::process::abort());`
     then `for i in 0..16 { ulid[i] = B32_ALPHABET.as_bytes()[rnd[i] as usize % 32] as u8; }`
     (keep the `% 32` modulo bias — do NOT "improve" it).
2. `pub fn ulid() -> String`: zeroed `[0u8; 27]`, call `ulidgen_r`, return
   `String::from_utf8_lossy(&buf[..26]).into_owned()`.

Faithfulness notes: keep the stateful-buffer semantics (the function reads the
previous ULID from its argument); keep recursion on Z-carry overflow; keep
`abort()` on getentropy failure.

### A3. `src/main.rs` — CLI (depends on `ulidgen` lib)
Port `src/ulidgen.c`:
- Parse `std::env::args().skip(1)`: `-n N` (next arg parsed as `i64`, default
  `n = 1`), `-t` sets `tflag = true`; unknown flag → usage to stderr, exit 1.
- `-t` mode: `io::stdin().lock()` + `read_line` loop; reuse ONE `let mut ulid =
  [0u8; 27];` buffer across `ulidgen_r` calls (preserves C same-ms increment);
  `write!(stdout, "{} {}", String::from_utf8_lossy(&ulid[..26]), line)`.
- `-n` mode: loop `n` times, `println!` each ULID (same reused buffer).
- Final `stdout.flush()`; on any write/flush error `std::process::exit(1)`
  (mirrors `exit(!!ferror(stdout))`). No `setvbuf` equivalent needed.

## Part B — test files (bottom-up dependency order)

### B1. `tests/test.rs` — integration tests (depends on `ulidgen` lib)
Port all four tests from `tests/test.c` (uncomment `test_ulid_structure`,
which was disabled in the C `main`):
- `is_valid_ulid(ulid: &str) -> bool`: `ulid.len() == 26` and every char is in
  `ulidgen::B32_ALPHABET`.
- `test_ulid_length`: `ulidgen::ulid().len() == 26`.
- `test_ulid_structure`: `is_valid_ulid(&ulidgen::ulid())`.
- `test_ulid_uniqueness`: two consecutive `ulidgen::ulid()` calls differ.
- `test_ulid_sortability`: generate one, `thread::sleep(Duration::from_millis(1))`
  + `from_micros(500)` (1.5 ms, as in C), generate second, assert
  `first < second` lexicographically.

## Verification
- `cargo check --all-targets` — no errors.
- `cargo test` — all 4 tests pass.
- `cargo run -- -n 3` — prints 3 ULIDs; `echo hi | cargo run -- -t` — prints
  `<ULID> hi`.
