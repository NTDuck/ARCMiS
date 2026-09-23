# ulidgen — C → Rust Translation Design

## 1. Source project analysis

**Project:** `ulidgen` — a small public-domain C utility that generates ULIDs
(Universally Unique Lexicographically Sortable Identifiers) or prefixes stdin
lines with them. Author: Leah Neukirchen (Void Linux).

### Files

| File | Role |
|------|------|
| `src/ulid.h` | Single declaration: `void ulidgen_r(char[27]);` |
| `src/ulid.c` | Core ULID generator (81 lines) |
| `src/ulidgen.c` | CLI entry point: `ulidgen [-n N \| -t]` |
| `tests/test.c` | 4 tests: length, structure, uniqueness, sortability |
| `Makefile` | Builds `ulid.o`, compiles `test_1` from `tests/test.c` + `src/ulid.c`, runs it |
| `README` | Man-page text (ulidgen(1)) |

### Core algorithm (`ulidgen_r`)

1. Crockford base32 alphabet: `0123456789ABCDEFGHJKMNPQRSTVWXYZ` (no I, L, O, U).
2. Get wall-clock time in **milliseconds** via `clock_gettime(CLOCK_REALTIME)`.
3. Encode the 48-bit timestamp into the first 10 chars by repeated
   `t /= 32; ulid[i] = alphabet[t % 32]` (i = 9..0). Tracks whether all 10
   chars are unchanged from the buffer's previous contents (`same` flag).
4. If `same` (same millisecond as previous call): **increment the random part
   in place** — scan from index 15 down, wrap `Z` → `0` with carry; if the
   carry runs past index 0, `nanosleep(1.234567 ms)` and **recurse**. If the
   char at the carry position is not in the alphabet (invalid), fall through
   to full re-randomization.
5. Otherwise: fill 16 random bytes via `getentropy` (abort on failure) and
   encode each byte as `alphabet[rnd[i] % 32]` into positions 10..25.
6. Buffer is 27 bytes: 26 chars + NUL terminator.

**Key semantic detail:** the function is *stateful via its argument* — it
reads the previous ULID from the caller's buffer to detect the same-millisecond
case and to increment. The CLI reuses one `char ulid[27]` across all calls.

### CLI (`ulidgen.c`)

- `getopt` with opts `n:` and `t`.
- `-n N`: print N ULIDs (default 1), one per line.
- `-t`: line-buffer stdout (`setvbuf(stdout, 0, _IOLBF, 0)`), read stdin with
  `getdelim`, print `<ulid> <line>` (line keeps its trailing newline).
- Exit status: `exit(!!ferror(stdout))` — nonzero if a write failed.

### Tests (`tests/test.c`)

- `test_ulid_length`: strlen == 26.
- `test_ulid_structure`: all chars in Crockford alphabet (currently commented
  out in `main`, but present).
- `test_ulid_uniqueness`: two consecutive ULIDs differ.
- `test_ulid_sortability`: after a 1.5 ms sleep, second ULID sorts after the
  first (`strcmp(ulid1, ulid2) < 0`).

### Build/test setup

Plain Makefile, no third-party C dependencies (libc only: `stdint.h`,
`stdlib.h`, `string.h`, `time.h`, `unistd.h`). Test command: compile and run
`test_1`.

## 2. Third-party dependency mapping

The C project has **zero third-party dependencies** (libc only). The idiomatic
Rust translation can also be **zero-dependency** using std:

| C facility | Rust std counterpart | Notes |
|------------|---------------------|-------|
| `clock_gettime(CLOCK_REALTIME)` | `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` | Gives `Duration` with secs + nanos; compute ms as `secs*1000 + nanos/1_000_000` |
| `getentropy(buf, len)` | `std::os::unix::fs::getentropy(&mut buf)` | Stable since Rust 1.70; Unix-only, matches the C target. Alternative: `getrandom` crate (not needed) |
| `nanosleep` | `std::thread::sleep(Duration::from_nanos(1_234_567))` | |
| `getopt` | manual `std::env::args()` parsing | Only two flags (`-n N`, `-t`); `clap` would be overkill. Manual parsing keeps the project dependency-free |
| `setvbuf(stdout, _IOLBF)` | nothing needed | Rust's `std::io::stdout()` is already line-buffered (`LineWriter`) |
| `getdelim` | `std::io::BufRead::read_line` | Preserves trailing newline |
| `atol` | `str::parse::<i64>()` | |
| `exit(!!ferror(stdout))` | check `Write` results / `flush()` error → `std::process::exit(1)` | |
| `abort()` on getentropy failure | `std::process::abort()` | Faithful to C behavior |

**Decision: no external crates.** `Cargo.toml` has an empty `[dependencies]`.
This is the most idiomatic choice for a zero-dependency C tool and keeps the
build hermetic.

## 3. Target project structure

```
ulidgen/
├── Cargo.toml          # package name "ulidgen", edition 2021, no deps
├── src/
│   ├── lib.rs          # core: pub fn ulidgen_r(ulid: &mut [u8; 27])
│   └── main.rs         # CLI: -n N / -t, reads stdin, prints
└── tests/
    └── test.rs         # integration tests (or #[cfg(test)] in lib.rs)
```

`Cargo.toml`:
```toml
[package]
name = "ulidgen"
version = "0.1.0"
edition = "2021"

[lib]
name = "ulidgen"
path = "src/lib.rs"

[[bin]]
name = "ulidgen"
path = "src/main.rs"
```

### `src/lib.rs` — faithful API

Mirror the C signature so the stateful-buffer semantics carry over:

```rust
pub const B32_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into `ulid` (26 chars + NUL), exactly like the C
/// `ulidgen_r(char[27])`. Reads the previous contents of `ulid` to detect
/// the same-millisecond case and increment the random part in place.
pub fn ulidgen_r(ulid: &mut [u8; 27]) { ... }
```

Implementation notes:
- Work on the byte slice; `ulid[26] = 0` (NUL) as in C.
- Timestamp: `SystemTime::now().duration_since(UNIX_EPOCH)` →
  `t = secs as u64 * 1000 + nanos / 1_000_000`.
- Timestamp encoding loop `for i in (0..10).rev()` with `t /= 32`,
  `same` flag, byte comparison against `B32_ALPHABET.as_bytes()`.
- Same-ms increment: scan `i` from 15 down while `buf[i] == b'Z'` → set `b'0'`;
  if `i < 0`: `thread::sleep(1_234_567 ns)` and recurse (`ulidgen_r(ulid)`);
  else if the byte is in the alphabet: advance to the next alphabet byte;
  otherwise fall through to re-randomization.
- Random fill: `let mut rnd = [0u8; 16]; std::os::unix::fs::getentropy(&mut rnd)
  .unwrap_or_else(|_| std::process::abort());` then
  `buf[i] = B32_ALPHABET.as_bytes()[rnd[i] as usize % 32] as u8`.
- Optionally also expose an ergonomic `pub fn ulid() -> String` wrapper
  (generate into a zeroed `[0u8; 27]`, return `String::from_utf8_lossy` of the
  first 26 bytes) — used by `main.rs` and tests.

### `src/main.rs` — CLI

- Parse `std::env::args().skip(1)`: support `-n N` (next arg is N, parse as
  `i64`, default 1) and `-t` flag; unknown flag → print usage to stderr,
  exit 1.
- `-t` mode: `let stdin = io::stdin(); let stdout = io::stdout();`
  `BufRead::read_line` loop; for each line call `ulidgen_r` on a reused
  `[0u8; 27]` buffer (preserves the C same-ms increment behavior), then
  `write!(stdout, "{} {}", ulid_str, line)`.
- `-n` mode: loop `n` times, `println!` each ULID.
- Final `flush()`; on any write/flush error `std::process::exit(1)`
  (mirrors `exit(!!ferror(stdout))`).

### Tests (`tests/test.rs` or `#[cfg(test)]` in `lib.rs`)

Port all four C tests to `#[test]` functions (run by `cargo test`):

1. `test_ulid_length` — generated string has length 26.
2. `test_ulid_structure` — every char is in the Crockford alphabet (uncomment
   it; it was disabled in the C `main` but is a valid test).
3. `test_ulid_uniqueness` — two consecutive ULIDs differ.
4. `test_ulid_sortability` — generate one, `thread::sleep(1.5 ms)`, generate
   second, assert `first < second` lexicographically.

Use the ergonomic `ulid() -> String` wrapper in tests for readability.

## 4. Risks and mitigations

1. **Stateful-buffer semantics.** The C function reads its output buffer for
   the previous ULID. A naive Rust rewrite as `fn ulid() -> String` would lose
   the same-millisecond increment path (tests would still pass, but behavior
   changes under rapid generation). Mitigation: keep the faithful
   `ulidgen_r(&mut [u8; 27])` as the core API and have the CLI reuse one
   buffer, exactly like the C `main`.
2. **`getentropy` is Unix-only** (`std::os::unix::fs`). The C source is also
   Unix-only (`getentropy`, `nanosleep`), so this is acceptable; document it.
   If portability were required, the `getrandom` crate would be the fallback.
3. **Recursion on Z-carry overflow** — preserved as-is; depth is bounded by
   the sleep, so no stack risk.
4. **`rnd[i] % 32` bias** — the C code uses modulo bias; keep it identical for
   behavioral fidelity (do *not* "improve" to unbiased sampling, which would
   change the distribution and could break byte-for-byte comparisons).
5. **Exit status on write error** — Rust `println!`/`write!` errors must be
   checked; use `Result` propagation or explicit `exit(1)` to mirror
   `exit(!!ferror(stdout))`.
6. **Line buffering** — Rust stdout is line-buffered by default, so `-t` mode
   behavior matches without `setvbuf` equivalent.
7. **Test flakiness** — `test_ulid_sortability` relies on a 1.5 ms sleep
   crossing a millisecond boundary; keep the same sleep as the C test.
   `test_ulid_uniqueness` can in theory collide only via the increment path,
   which guarantees distinctness — same as C.

## 5. Build & test

- Build: `cargo build` (produces `target/debug/ulidgen`).
- Test: `cargo test` (runs the ported unit/integration tests).
- No external crates, no network needed for the build.
