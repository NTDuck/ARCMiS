# ulidgen — C → Rust Translation Plan

Source: C project `ulidgen` (public domain). Target: single Rust crate in this
workspace. Test command: `cargo test`.

## 1. Fragment extraction (source inventory)

| Source file | Fragment | Kind |
|---|---|---|
| `src/ulid.h` | `void ulidgen_r(char[27]);` | declaration (no code) |
| `src/ulid.c` | `ulidgen_r(char ulid[27])` | function — core ULID generation |
| `src/ulidgen.c` | `main(int argc, char *argv[])` | function — CLI (`-n N` / `-t`) |
| `tests/test.c` | `is_valid_ulid(const char *)` | helper function |
| `tests/test.c` | `test_ulid_length()` | test |
| `tests/test.c` | `test_ulid_structure()` | test (defined, not called by C `main`) |
| `tests/test.c` | `test_ulid_uniqueness()` | test |
| `tests/test.c` | `test_ulid_sortability()` | test |
| `tests/test.c` | `main()` | test runner (replaced by `cargo test`) |
| `Makefile`, `README`, `coverage_report.json` | build/docs/coverage | not translated (cargo replaces Makefile) |

## 2. Name mapping (C → Rust)

| C symbol | Rust symbol | Changed? | Why |
|---|---|---|---|
| `ulidgen_r` | `ulidgen_r` | no | preserved; `pub fn ulidgen_r(ulid: &mut [u8; 27])` in `src/lib.rs` |
| `b32alphabet` (local) | `B32_ALPHABET` | yes | Rust `const` convention (SCREAMING_SNAKE); promoted to `pub const` in `lib.rs` |
| `main` | `main` | no | preserved in `src/main.rs` |
| `is_valid_ulid` | `is_valid_ulid` | no | preserved (private helper in `tests/test.rs`) |
| `test_ulid_length` | `ulid_length` | yes | Rust `#[test]` naming; `test_` prefix dropped (cargo test convention) |
| `test_ulid_structure` | `ulid_structure` | yes | same as above |
| `test_ulid_uniqueness` | `ulid_uniqueness` | yes | same as above |
| `test_ulid_sortability` | `ulid_sortability` | yes | same as above |
| `main` (in `tests/test.c`) | — | removed | `cargo test` is the runner; no test `main` needed |
| `getentropy` | `getrandom::fill` | yes | crate API (design decision) |
| `clock_gettime` | `SystemTime::now()` | yes | std API |
| `nanosleep` | `std::thread::sleep` | yes | std API |
| `getopt` | manual `std::env::args` loop | yes | zero-dep, mirrors `getopt("n:t")` |
| `getdelim` | `BufRead::read_line` | yes | std API (keeps newline → byte-exact `-t` output) |
| `setvbuf(_IOLBF)` | — | removed | Rust `Stdout` is line-buffered on TTYs |
| `abort()` | `std::process::abort()` | yes | same semantics |
| `assert` | `assert!` / `assert_eq!` / `assert_ne!` | yes | Rust macros |

## 3. Skeleton (already written, compiles)

- `Cargo.toml` — package `ulidgen`, dep `getrandom = "0.2"`, empty `[workspace]`
  table to detach from the enclosing ARCMiS cargo workspace.
- `src/lib.rs` — `B32_ALPHABET` const + `pub fn ulidgen_r(ulid: &mut [u8; 27])` stub (`todo!`).
- `src/main.rs` — `main` with the full arg-parse skeleton and two `todo!` mode bodies.
- `tests/test.rs` — `is_valid_ulid` helper + four `#[test]` stubs (`todo!`).

## 4. Implementation plan

### Part A — source files (bottom-up dependency order)

1. **`src/lib.rs`** — implement `ulidgen_r` (port of `src/ulid.c`):
   - ms timestamp via `SystemTime::now().duration_since(UNIX_EPOCH)` →
     `secs*1000 + nanos/1_000_000`;
   - encode 10 base32 digits big-endian (`for i in (0..10).rev()`, `t % 32`, `t /= 32`),
     tracking `same` against the caller's previous buffer contents;
   - same-millisecond path: carry-increment from index 15 (`'Z'`→`'0'`), on carry-off
     `thread::sleep(Duration::from_nanos(1_234_567))` + recurse; if the char is not in
     the alphabet, fall through to re-randomization;
   - random path: 16 bytes via `getrandom::fill` (on error `std::process::abort()`),
     map `B32_ALPHABET[b % 32]` — **preserve the modulo bias**;
   - use `split_at_mut` for the `buf` slice; keep `ulid[26] = 0`.
   - Depends on: nothing (only std + getrandom).

2. **`src/main.rs`** — implement the two mode bodies (port of `src/ulidgen.c`):
   - tag mode: `stdin.lock().read_line` loop (keeps newline → byte-exact `"%s %s"`),
     `ulidgen_r(&mut ulid)`, write `"{ulid} {line}"`;
   - generate mode: loop `0..n.max(0)`, `ulidgen_r`, `writeln!` the 26 chars;
   - keep the existing arg parse, flush, and `exit(0/1)` on flush error.
   - Depends on: `src/lib.rs` (`ulidgen_r`).

### Part B — test files (bottom-up dependency order)

1. **`tests/test.rs`** — implement the four `#[test]`s (port of `tests/test.c`):
   - `ulid_length`: generate into `[0u8; 27]`, assert the 26-char string has len 26;
   - `ulid_structure`: assert `is_valid_ulid` (all chars in alphabet);
   - `ulid_uniqueness`: two consecutive ULIDs differ (`assert_ne!`);
   - `ulid_sortability`: generate, `sleep(1.5 ms)`, generate, assert second > first.
   - Depends on: `src/lib.rs`.

### Verification

- `cargo build` — binary `ulidgen` builds.
- `cargo test` — all 4 tests pass.
- Smoke: `ulidgen -n 3` prints 3 ULIDs; `printf 'a\nb\n' | ulidgen -t` prefixes lines.
