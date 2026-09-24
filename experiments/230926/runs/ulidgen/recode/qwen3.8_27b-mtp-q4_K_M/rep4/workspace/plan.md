# ulidgen — C → Rust Translation Plan

Source: C CLI `ulidgen` (public domain). Target: Cargo package `ulidgen`
(edition 2021), test command `cargo test`.

## 1. Fragment extraction (source → target)

| Source fragment | Location | Target fragment | Target location |
|---|---|---|---|
| `ulidgen_r` (declaration) | `src/ulid.h` | `pub fn ulidgen_r(ulid: &mut [u8; 27])` | `src/lib.rs` |
| `ulidgen_r` (definition) | `src/ulid.c` | `pub fn ulidgen_r(ulid: &mut [u8; 27])` | `src/lib.rs` |
| `b32alphabet` (static const) | `src/ulid.c` | `const B32: &[u8; 32]` | `src/lib.rs` |
| `main` (CLI) | `src/ulidgen.c` | `fn main()` | `src/main.rs` |
| `is_valid_ulid` (helper) | `tests/test.c` | `fn is_valid_ulid(ulid: &[u8]) -> bool` | `src/lib.rs` `#[cfg(test)] mod tests` |
| `test_ulid_length` | `tests/test.c` | `#[test] fn test_ulid_length()` | `src/lib.rs` `#[cfg(test)] mod tests` |
| `test_ulid_structure` | `tests/test.c` | `#[test] fn test_ulid_structure()` | `src/lib.rs` `#[cfg(test)] mod tests` |
| `test_ulid_uniqueness` | `tests/test.c` | `#[test] fn test_ulid_uniqueness()` | `src/lib.rs` `#[cfg(test)] mod tests` |
| `test_ulid_sortability` | `tests/test.c` | `#[test] fn test_ulid_sortability()` | `src/lib.rs` `#[cfg(test)] mod tests` |
| `main` (test runner) | `tests/test.c` | `cargo test` harness (no port needed) | — |

## 2. Name mapping

Preserved names (identical in Rust): `ulidgen_r`, `main`, `is_valid_ulid`,
`test_ulid_length`, `test_ulid_structure`, `test_ulid_uniqueness`,
`test_ulid_sortability`.

Changed names / replaced constructs:

| C name | Rust name | Why |
|---|---|---|
| `b32alphabet` | `B32` | Rust const naming convention (SCREAMING_SNAKE); also `char*` → `&[u8; 32]` |
| `getentropy(rnd, 16)` | `getrandom::getrandom(&mut rnd)` | crate counterpart; `abort()` on failure → `expect`/`panic!` |
| `clock_gettime(CLOCK_REALTIME)` | `SystemTime::now().duration_since(UNIX_EPOCH)` | std counterpart |
| `nanosleep(1.234567 ms)` | `std::thread::sleep(Duration::from_nanos(1_234_567))` | std counterpart; keep exact 1.234567 ms |
| `getopt("n:t")` | manual loop over `std::env::args().skip(1)` | no getopt in std; `atol` → `i64::from_str` |
| `getdelim(..., '\n', stdin)` | `std::io::stdin().lock().read_line(&mut line)` | std counterpart; `read_line` keeps the `\n` like `getdelim` |
| `setvbuf(stdout, _IOLBF)` | explicit `flush()` after each line in `-t` mode | Rust stdout is block-buffered when piped; per-line flush mirrors line buffering |
| `puts(ulid)` | `writeln!(stdout, "{}", ...)` | std counterpart |
| `exit(!!ferror(stdout))` | `process::exit(1)` on write error, else return | same semantics: 0 on success, 1 on write failure |
| `char ulid[27]` (26 chars + NUL) | `[u8; 27]` (26 ASCII chars + NUL at index 26) | keep the 27-byte buffer contract so the stateful increment path works |

## 3. Skeleton status

Skeleton files already exist and compile (`cargo build` / `cargo test` pass
with stubs):

- `Cargo.toml` — package `ulidgen`, dependency `getrandom = "0.3"`.
- `src/lib.rs` — `B32` const, `pub fn ulidgen_r` stub with TODO notes,
  `#[cfg(test)] mod tests` with `is_valid_ulid` stub and 4 test stubs.
- `src/main.rs` — `fn main` stub with TODO notes.

## 4. Implementation plan

### Part A — source files (bottom-up dependency order)

1. **`src/lib.rs`** (port of `src/ulid.c` + `src/ulid.h`)
   - Implement `ulidgen_r(ulid: &mut [u8; 27])`:
     1. `ulid[26] = 0` (NUL terminator).
     2. `t = secs*1000 + nanos/1_000_000` from `SystemTime::now()`.
     3. Encode 10 timestamp chars for `i = 9..=0`: compare against the
        PRE-EXISTING `ulid[i]` to set `same`; write `B32[t % 32]`; `t /= 32`.
     4. If `same`: walk `i = 15..=0` over `ulid[10..]` while char == `b'Z'`
        → set `b'0'`; if `i` underflows (all Z): `thread::sleep(1.234567 ms)`
        and recurse `ulidgen_r(ulid)`; else if char is in `B32` bump to its
        successor and return; if char not in `B32` fall through.
     5. Else: `getrandom::getrandom(&mut rnd[16])` (panic on error, mirrors
        `abort()`); `ulid[10+i] = B32[rnd[i] % 32]` — **preserve the biased
        `% 32` encoding, do not "fix" it**.
   - Preserve the stateful-buffer contract: the previous ULID must remain in
     the caller's buffer for the increment path.
   - Implement `is_valid_ulid` and the 4 tests in `#[cfg(test)] mod tests`
     (see Part B — they live in this same file, so fill them together).

2. **`src/main.rs`** (port of `src/ulidgen.c`; depends on `lib.rs`)
   - Parse `std::env::args().skip(1)`: `-n N` (`i64`, parse/unknown flag →
     usage to stderr + `process::exit(1)`), `-t` tag mode.
   - Tag mode: one `[0u8; 27]` buffer reused across lines; per line:
     `ulidgen_r(&mut ulid)`, write `"<26 chars> <line>"` (line keeps its
     `\n`), `flush()` after each line (mirrors `setvbuf(_IOLBF)`).
   - Generate mode: loop `0..n`, `ulidgen_r(&mut ulid)`, print the 26 chars
     + newline.
   - Any stdout write error → `process::exit(1)` (mirrors
     `exit(!!ferror(stdout))`).

### Part B — test files (bottom-up dependency order)

1. **`#[cfg(test)] mod tests` in `src/lib.rs`** (port of `tests/test.c`)
   - `is_valid_ulid(ulid: &[u8]) -> bool`: length 26, every byte in `B32`.
   - `test_ulid_length`: generate into `[0u8; 27]`, assert 26 chars + NUL.
   - `test_ulid_structure`: assert `is_valid_ulid(&ulid[..26])` (keep
     ENABLED — it is commented out in the C `main` but present in the file).
   - `test_ulid_uniqueness`: two buffers, two consecutive calls, assert
     they differ.
   - `test_ulid_sortability`: generate, `thread::sleep(1.5 ms)`, generate,
     assert `ulid1 < ulid2` lexicographically.

### Verification

- `cargo build` — no errors.
- `cargo test` — all 4 tests pass.
- Smoke: `cargo run -- -n 3` prints 3 ULIDs;
  `printf 'a\nb\n' | cargo run -- -t` prefixes each line.
