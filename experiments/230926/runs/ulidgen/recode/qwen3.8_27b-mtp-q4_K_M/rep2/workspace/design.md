# ulidgen — C → Rust Translation Design

## 1. Source project analysis

**Project:** `ulidgen` — a small public-domain CLI tool that generates ULIDs
(Universally Unique Lexicographically Sortable Identifiers) or prefixes stdin
lines with them. Author: Leah Neukirchen (Void Linux).

### Files

| File | Role |
|---|---|
| `src/ulid.c` / `src/ulid.h` | Core library: `void ulidgen_r(char ulid[27])` — fills a 27-byte buffer (26 chars + NUL) with a ULID |
| `src/ulidgen.c` | CLI entry point: `ulidgen [-n N \| -t]` |
| `tests/test.c` | 4 tests: length, structure (commented out in main), uniqueness, sortability |
| `Makefile` | Builds `ulid.o` and a `test_1` binary; coverage flags; install to `/usr/local` |
| `README` | Man-page text (ulidgen.1 rendered) |

### Core algorithm (`ulidgen_r`)

1. `clock_gettime(CLOCK_REALTIME)` → millisecond timestamp `t`.
2. Encode the 40-bit timestamp into the first 10 chars using repeated
   `t % 32` / `t /= 32` over the Crockford base32 alphabet
   `"0123456789ABCDEFGHJKMNPQRSTVWXYZ"` (no I, L, O, U).
3. Track `same`: whether all 10 timestamp chars are unchanged from the
   previous call (i.e. same millisecond).
4. If `same`: increment the 16-char random part (indices 10..25) in place:
   - scan from index 25 down while char == 'Z', setting to '0';
   - if all wrapped (i < 0): `nanosleep(1234567 ns)` and **recurse**;
   - if the char is in the alphabet: advance to the next alphabet char;
   - if the char is *not* in the alphabet (corrupt buffer): fall through and
     re-randomize.
5. Otherwise: `getentropy(rnd, 16)` (abort on failure) and encode each byte
   as `b32alphabet[rnd[i] % 32]`.

### CLI behavior (`main`)

- `getopt` with `"n:t"`: `-n N` (default 1, `atol`), `-t` flag.
- `-t`: `setvbuf(stdout, 0, _IOLBF, 0)` (line-buffered), then
  `getdelim` loop printing `"%s %s"` (ULID, space, line — line keeps its
  trailing newline).
- default: print `n` ULIDs, one per line (`puts`).
- Exit status: `exit(!!ferror(stdout))` — non-zero if a write error occurred.

### Tests

- `test_ulid_length`: strlen == 26.
- `test_ulid_structure`: all chars in Crockford alphabet (defined but not
  called from main — we will enable it in the Rust port).
- `test_ulid_uniqueness`: two consecutive ULIDs differ.
- `test_ulid_sortability`: after a 1.5 ms `nanosleep`, second ULID sorts
  after the first (lexicographic `<`).

### Build/test setup

Plain `Makefile`, no package manager, no third-party dependencies — pure
POSIX libc (`getopt`, `getdelim`, `getentropy`, `clock_gettime`,
`nanosleep`). Test command for the target: `cargo test`.

## 2. Dependency mapping (C → Rust)

The C project has **zero third-party dependencies**; every capability comes
from libc. Mapping to idiomatic Rust:

| C capability | Source | Rust counterpart | Notes |
|---|---|---|---|
| `getentropy(2)` | `src/ulid.c` | **`getrandom`** crate (`getrandom::getrandom(&mut buf)`) | `std` does not expose `getentropy`; `getrandom` is the idiomatic, zero-alloc, no-OS-alloc crate. Alternative: `rand::rngs::OsRng`. We choose `getrandom` for minimalism. |
| `clock_gettime(CLOCK_REALTIME)` | `src/ulid.c` | `std::time::SystemTime::now()` + `Duration::duration_since(UNIX_EPOCH)` | std only; millisecond precision is sufficient. |
| `nanosleep` | `src/ulid.c`, `tests/test.c` | `std::thread::sleep(Duration)` | std only. |
| `getopt` | `src/ulidgen.c` | **`clap`** (derive API) | Idiomatic CLI parsing. `-n N` (default 1), `-t` flag. (Could be done with `std::env::args`, but clap is the idiomatic choice and matches the man-page interface exactly.) |
| `getdelim` / `setvbuf(_IOLBF)` | `src/ulidgen.c` | `std::io::BufRead::lines()` on `stdin.lock()`; per-line `flush()` | Line buffering achieved by flushing after each printed line. |
| `exit(!!ferror(stdout))` | `src/ulidgen.c` | propagate `io::Result`; `std::process::exit(1)` on write error | Rust: check `flush()`/write results and exit non-zero. |
| `assert` | `tests/test.c` | `assert!` / `assert_eq!` in `#[test]` fns | Direct port. |

**Cargo.toml dependencies:** `clap` (with `derive` feature), `getrandom`.
Both are stable, widely used crates.

## 3. Target project structure

```
ulidgen/
├── Cargo.toml
├── src/
│   ├── lib.rs        # ulidgen_r() core (port of src/ulid.c) + tests module
│   └── main.rs       # CLI (port of src/ulidgen.c)
└── tests/
    └── ulid.rs       # integration tests (port of tests/test.c)
```

### `src/lib.rs`

- `const B32_ALPHABET: [u8; 32]` — Crockford alphabet bytes.
- `pub fn ulidgen_r(buf: &mut [u8; 27])` — faithful port of the C function:
  - `buf[26] = 0` (NUL terminator, kept for API fidelity).
  - Millisecond timestamp via `SystemTime`.
  - Timestamp encoding loop `i in (0..10).rev()`, tracking `same`.
  - In-place increment of `buf[10..26]` with the same 'Z'→'0' carry logic;
    overflow → `thread::sleep(Duration::from_nanos(1_234_567))` and recurse
    (or loop; recursion depth is bounded by the 16-char counter, keep
    recursion for fidelity).
  - Re-randomize path: `getrandom(&mut rnd)` (16 bytes), map
    `rnd[i] % 32` → alphabet byte.
- Convenience wrapper `pub fn ulid() -> String` (26 chars) for the CLI/tests.
- `#[cfg(test)] mod tests` — unit tests ported from `tests/test.c`:
  - `test_ulid_length` (26 chars),
  - `test_ulid_structure` (all chars in alphabet — **enabled**, it was
    commented out in the C main),
  - `test_ulid_uniqueness`,
  - `test_ulid_sortability` (1.5 ms sleep between generations).

### `src/main.rs`

- `clap::Parser` struct:
  - `-n, --count <N>`: `u64`, default 1 (C used `atol`; we use `u64` and
    treat 0 as zero output — matches `for (i=0; i<n; i++)`).
  - `-t, --tag`: `bool` flag.
- `-t` mode: read `stdin` lines with `BufRead::lines()`, print
  `"{ulid} {line}\n"` (note: `lines()` strips the newline, so re-append
  `'\n'` to preserve the C `getdelim` semantics where the line keeps its
  trailing newline), flush after each line (line-buffering equivalent).
- Default mode: print `n` ULIDs, one per line.
- Error handling: on any stdout write/flush error, `eprintln!` and
  `std::process::exit(1)` (equivalent of `exit(!!ferror(stdout))`).

### `tests/ulid.rs`

Integration tests mirroring `tests/test.c` against the public API
(`ulidgen::ulid()` / `ulidgen_r`): length, structure, uniqueness,
sortability. This keeps `cargo test` running both unit and integration
tests.

### Cargo.toml

```toml
[package]
name = "ulidgen"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
getrandom = "0.2"
```

## 4. Translation risks

1. **`getentropy` → `getrandom`:** `getrandom` returns `Result`; the C code
   calls `abort()` on failure. Rust: `expect("getrandom failed")` or
   `unwrap()` preserves the abort-on-failure semantics.
2. **In-place buffer mutation:** C mutates a caller-owned 27-byte buffer and
   relies on prior contents for the "same millisecond" detection. Rust's
   `&mut [u8; 27]` preserves this exactly; the CLI must keep a persistent
   buffer across calls (as C's `main` does with `char ulid[27]`).
   **Do not** regenerate a fresh buffer per call in the CLI, or the
   increment-on-same-ms path never triggers.
3. **`rnd[i] % 32` modulo bias:** the C code uses `byte % 32`, which is
   slightly biased (256 % 32 == 0, so actually *unbiased* — 256 is a multiple
   of 32; no risk, keep the same expression for fidelity).
4. **Line semantics in `-t` mode:** C's `getdelim` keeps the trailing
   newline; Rust's `lines()` strips it. Must re-append `'\n'` to match
   output byte-for-byte. Also C prints `"%s %s"` — ULID, single space, line.
5. **Sortability test flakiness:** relies on a 1.5 ms sleep guaranteeing a
   different millisecond. Same risk in Rust; keep the 1.5 ms sleep.
   (If the two calls land in the same ms, the increment path still makes
   the second ULID larger, so the test is actually robust either way.)
6. **Exit status:** C exits `1` only on stdout write error. Rust must not
   let clap's own error paths change the contract; clap already exits 2 on
   usage errors, which is acceptable (C's getopt also exits non-zero on
   usage errors).
7. **Recursion on counter overflow:** bounded (max 16 levels of 'Z' wrap),
   safe to port as recursion; alternatively convert to a loop.
8. **`atol` vs typed parsing:** C silently accepts garbage for `-n`; clap
   will reject non-numeric input with an error. This is a behavior
   improvement, not a regression.

## 5. Verification

- `cargo test` runs unit tests (lib.rs) + integration tests (tests/ulid.rs).
- Manual smoke: `cargo run -- -n 3` prints 3 ULIDs;
  `echo hello | cargo run -- -t` prints `<ULID> hello`.
- Output format: 26 Crockford-base32 chars, lexicographically sortable.
