# ulidgen — C → Rust Translation Design

## 1. Source project overview

`ulidgen` is a tiny public-domain C utility (by Leah Neukirchen) that generates
ULIDs (Universally Unique Lexicographically Sortable Identifiers) or prefixes
stdin lines with them.

### Files

| File | Role |
|---|---|
| `src/ulid.h` | One declaration: `void ulidgen_r(char[27]);` |
| `src/ulid.c` | Core: fills a 27-byte buffer (26 chars + NUL) with a ULID |
| `src/ulidgen.c` | CLI: `ulidgen [-n N] [-t]` |
| `tests/test.c` | 4 tests (length, structure, uniqueness, sortability); structure test is commented out in `main` |
| `Makefile` | Builds `ulid.o`, compiles+runs `test_1` via `gcc`, installs binary + man page |
| `README` | Generated man page (ulidgen.1) |

### Core algorithm (`ulidgen_r`)

1. `clock_gettime(CLOCK_REALTIME)` → millisecond timestamp `t`.
2. Encode the 10 most significant base-32 digits (Crockford alphabet
   `0123456789ABCDEFGHJKMNPQRSTVWXYZ`) into `ulid[0..10]`, dividing `t` by 32
   each step. If **every** digit already matched the previous value in the
   buffer, `same` stays 1.
3. If `same` (same millisecond as the previous call, buffer was reused by the
   caller): increment the 16-char random part in place, right to left, wrapping
   `'Z'` → `'0'`. If all 16 chars were `'Z'`, sleep 1.234567 ms and recurse.
   If any random char is not in the alphabet (corrupt buffer), fall through to
   full re-randomization.
4. Otherwise: `getentropy(rnd, 16)` (abort on failure), then
   `buf[i] = alphabet[rnd[i] % 32]` for i in 0..16 (note: deliberate `% 32`
   modulo bias — keep it for fidelity).

Key subtlety: **uniqueness within the same millisecond relies on the caller
reusing the same buffer** so the function can detect "same timestamp" and
increment. The Rust port must preserve this buffer-reuse contract.

### CLI behavior (`ulidgen.c`)

- `getopt("n:t")`: `-n N` (default 1), `-t` tag mode.
- `-t`: `setvbuf(stdout, _IOLBF)` (line-buffered), `getdelim` loop,
  `printf("%s %s", ulid, line)` — line includes its trailing newline.
- default: loop `n` times, `puts(ulid)`.
- Exit status: `exit(!!ferror(stdout))` — 0 on success, 1 on write error.

### Tests (`tests/test.c`)

- `test_ulid_length`: strlen == 26.
- `test_ulid_structure`: all chars in Crockford alphabet (commented out in main, but present).
- `test_ulid_uniqueness`: two consecutive ULIDs differ.
- `test_ulid_sortability`: ULID after a 1.5 ms sleep sorts after the first.

## 2. Dependency mapping (C → Rust)

| C source facility | Rust counterpart | Notes |
|---|---|---|
| `getentropy(2)` (unistd.h) | **`getrandom` crate** (v0.3.x) | Direct syscall wrapper, `getrandom::getrandom(&mut buf)`. Alternative: `rand` crate, but `getrandom` is the idiomatic 1:1 counterpart. |
| `clock_gettime(CLOCK_REALTIME)` | `std::time::SystemTime::now()` | No crate needed. Convert to ms since UNIX epoch via `Duration::as_millis()`. |
| `nanosleep` | `std::thread::sleep(Duration)` | No crate needed. |
| `getopt` (unistd.h) | **`getopts` crate** (v0.5.x) | Direct POSIX-getopt counterpart (`getopts::Options`, `optopt("n")`, `optflag("t")`). Alternative: `clap` (heavier, different UX). Manual parsing is also acceptable; `getopts` is the idiomatic match. |
| `getdelim` | `std::io::BufRead::read_line` | Std. |
| `setvbuf(_IOLBF)` | `std::io::LineWriter` | Std. |
| `atol` | `str::parse::<i64>()` | Std. |
| `ferror`/`exit` | check `io::Result` of writes; `std::process::exit(1)` on error | Std. |
| `assert.h` tests | `assert!` / `assert_eq!` in `#[test]` fns | Std. |

**Cargo.toml dependencies:** `getrandom = "0.3"` (with `std` feature),
`getopts = "0.5"`. Everything else is std.

## 3. Target project structure

```
ulidgen/
├── Cargo.toml
├── src/
│   ├── lib.rs        # pub mod ulid;  (exposes ulidgen_r for tests)
│   ├── ulid.rs       # core ULID generation (port of src/ulid.c)
│   └── main.rs       # CLI (port of src/ulidgen.c)
└── tests/
    └── test.rs       # port of tests/test.c
```

### `src/ulid.rs` — API

Preserve the C contract (caller reuses the buffer so same-ms increment works):

```rust
pub const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Fill `ulid` (26 chars + NUL, mirroring the C char[27]) with a new ULID.
/// The previous contents are used to detect a same-millisecond repeat and
/// to increment the random part in place, exactly like the C original.
pub fn ulidgen_r(ulid: &mut [u8; 27])
```

Implementation notes:
- `ulid[26] = 0` (NUL) as in C.
- Timestamp: `SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64`.
- Timestamp loop: `for i in (0..10).rev() { if ulid[i] != alphabet[(t % 32) as usize] { ulid[i] = ...; same = false; } t /= 32; }` — careful to mirror C's `same` flag semantics (starts `true`, cleared on any mismatch).
- Same-ms branch: walk `buf[15..0]`; while `buf[i] == b'Z'` set `b'0'`; if `i` underflows (all Z), `thread::sleep(Duration::from_nanos(1_234_567))` and recurse `ulidgen_r(ulid)`.
- Increment: find position of `buf[i]` in the alphabet; if found, set to next char; if not found (invalid char), fall through to re-randomize.
- Random: `let mut rnd = [0u8; 16]; getrandom(&mut rnd).unwrap_or_else(|_| std::process::abort());` then `buf[i] = alphabet[rnd[i] as usize % 32]` — **keep the `% 32` bias** for behavioral fidelity.

### `src/main.rs` — CLI

- Parse with `getopts`: option `n` (takes value, default 1), flag `t`.
- Keep a `let mut ulid = [0u8; 27];` buffer reused across calls (critical for
  same-ms uniqueness).
- `-t` mode: wrap stdout in `LineWriter`; loop `stdin.lock().read_line(&mut line)`;
  print `ulid[..26] as str` + b" " + line (line keeps its `\n`).
- Default mode: `n` iterations, print `ulid` + newline.
- On any write error: `eprintln!` optional, `std::process::exit(1)` (mirrors
  `exit(!!ferror(stdout))`).
- Convert bytes to `str` safely: the buffer only ever contains alphabet chars
  and NUL, so `std::str::from_utf8(&ulid[..26]).unwrap()` is fine.

### `tests/test.rs`

Port all four tests (including the commented-out structure test — enable it,
it passes):

```rust
use ulidgen::ulidgen_r;

fn is_valid_ulid(ulid: &str) -> bool { /* len 26, all chars in alphabet */ }

#[test] fn test_ulid_length() { let mut u = [0u8; 27]; ulidgen_r(&mut u); assert_eq!(std::str::from_utf8(&u[..26]).unwrap().len(), 26); }
#[test] fn test_ulid_structure() { ... assert!(is_valid_ulid(...)); }
#[test] fn test_ulid_uniqueness() { /* two consecutive calls differ */ }
#[test] fn test_ulid_sortability() { /* sleep 1.5 ms between calls; assert u1 < u2 */ }
```

Note: `cargo test` runs tests in parallel threads by default; each test uses
its own buffer, so there is no cross-test interference. The sortability test
needs `std::thread::sleep(Duration::from_millis(1))` + a small extra margin
(the C test sleeps 1.5 ms; use `Duration::from_micros(1_500)` to match).

### Cargo.toml

```toml
[package]
name = "ulidgen"
version = "0.1.0"
edition = "2021"

[dependencies]
getrandom = "0.3"
getopts = "0.5"
```

## 4. Risks & mitigations

1. **Buffer-reuse contract** — the biggest semantic risk. If the Rust port
   returns a fresh `String` each call, same-millisecond uniqueness breaks
   (the `test_ulid_uniqueness` test would be flaky). Mitigation: keep the
   `&mut [u8; 27]` in-out API exactly like C.
2. **`same` flag edge case** — C sets `same = 0` only when a digit *differs*;
   a fresh zeroed buffer has all digits differing, so `same` is 0 and the
   random path is taken. The Rust port must initialize `same = true` and clear
   on mismatch, not on match.
3. **All-'Z' wraparound recursion** — must recurse after the 1.234567 ms
   sleep, not loop; recursion depth is bounded in practice.
4. **Invalid-char fallthrough** — if the random part contains a char outside
   the alphabet, C re-randomizes instead of incrementing. Preserve this branch.
5. **Modulo bias (`rnd[i] % 32`)** — deliberately non-uniform; do not "fix"
   it with rejection sampling, or behavior diverges from the reference.
6. **Exit status** — C exits 1 only on stdout write error. Rust panics on
   `unwrap` would exit 101; handle write errors explicitly with `exit(1)`.
7. **Line handling in `-t` mode** — `getdelim` keeps the trailing newline and
   handles a final line without newline; `read_line` behaves the same. Print
   the line verbatim after `ulid + " "`.
8. **`getrandom` failure** — C calls `abort()`; mirror with
   `unwrap_or_else(|_| std::process::abort())`.
9. **Parallel test execution** — tests are independent (own buffers), but the
   sortability test relies on wall-clock ordering; 1.5 ms sleep is the same
   margin as the C test, acceptable.

## 5. Build & test

- Build: `cargo build`
- Test: `cargo test` (required test command)
- Run: `cargo run -- -n 3`, `echo hi | cargo run -- -t`
