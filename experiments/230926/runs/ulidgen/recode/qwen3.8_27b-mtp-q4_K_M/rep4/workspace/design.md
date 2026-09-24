# ulidgen — C → Rust Translation Design

## 1. Source project analysis

**Project:** `ulidgen` — a tiny CLI that generates ULIDs (Universally Unique
Lexicographically Sortable Identifiers) or prefixes stdin lines with them.
Public domain (CC0), by Leah Neukirchen.

### Files

| File | Role |
|---|---|
| `src/ulid.h` | Single public API: `void ulidgen_r(char ulid[27]);` |
| `src/ulid.c` | ULID generation core (timestamp + random, in-place increment) |
| `src/ulidgen.c` | CLI: `ulidgen [-n N] [-t]` |
| `tests/test.c` | 4 tests (length, structure [disabled], uniqueness, sortability) |
| `Makefile` | Builds `ulid.o`, compiles+runs `test_1`, installs binary + man page |

### Core algorithm (`ulidgen_r`)

1. Crockford base32 alphabet: `0123456789ABCDEFGHJKMNPQRSTVWXYZ` (no I, L, O, U).
2. Get wall-clock time via `clock_gettime(CLOCK_REALTIME)`, convert to
   **milliseconds** since epoch: `tv.tv_sec*1000 + tv.tv_nsec/1000000`.
3. Encode the 48-bit timestamp into the first 10 chars (indices 9..0) by
   repeated `t /= 32; ulid[i] = alphabet[t % 32]`.
4. **Same-millisecond handling:** if all 10 timestamp chars equal the
   *previous* ULID still sitting in the caller's buffer (`same == 1`),
   increment the 16-char random part in place:
   - walk from index 15 down while chars are `'Z'`, resetting them to `'0'`;
   - if all 16 were `'Z'` (overflow): `nanosleep(1.234567 ms)` and **recurse**;
   - otherwise bump the char to its successor in the alphabet (via
     `strchr`); if the char is not in the alphabet, fall through to
     full re-randomization.
5. Otherwise: fill 16 random bytes via `getentropy` (abort on failure) and
   encode each byte as `alphabet[rnd[i] % 32]` (note: **biased** mod-32
   encoding — must be preserved faithfully, not "fixed").
6. Buffer is 27 bytes: 26 chars + NUL.

Key subtlety: the function is **stateful through the caller's buffer** — the
previous ULID must remain in the buffer for the increment path to work. The
CLI reuses one buffer across all iterations.

### CLI (`ulidgen.c`)

- `getopt` with `n:` and `t`.
- `-t`: force line-buffered stdout (`setvbuf(_IOLBF)`), then for each line
  from stdin (`getdelim`), print `<ulid> <line>` (line keeps its `\n`).
- default / `-n N`: print N ULIDs, one per line (`puts`).
- Exit status: `exit(!!ferror(stdout))` → 0 on success, 1 if a write failed.

### Tests (`tests/test.c`)

- `test_ulid_length`: strlen == 26.
- `test_ulid_structure`: all chars in Crockford alphabet (currently
  commented out in `main`, but present).
- `test_ulid_uniqueness`: two consecutive ULIDs differ.
- `test_ulid_sortability`: ULID after a 1.5 ms sleep sorts after the first.

### Build/test

Plain `gcc` + Makefile; no third-party C dependencies (libc only:
`stdint.h`, `stdlib.h`, `string.h`, `time.h`, `unistd.h`).

## 2. Dependency mapping (C → Rust)

The C project has **no third-party dependencies** — only libc. Rust
counterparts:

| C facility | Rust counterpart | Notes |
|---|---|---|
| `getentropy(2)` | `getrandom` crate (`getrandom::getrandom(&mut buf)`) | Direct syscall wrapper; idiomatic. Alternative: `rand::rngs::OsRng` (pulls in `rand` + `getrandom`). Prefer `getrandom` alone — minimal. |
| `clock_gettime(CLOCK_REALTIME)` | `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` | std, no crate needed. |
| `nanosleep` | `std::thread::sleep(Duration)` | std. |
| `getopt` | manual `std::env::args()` parsing (or `clap`) | Only two flags; manual parsing keeps zero deps. `clap` acceptable if preferred. |
| `getdelim` / `getline` | `std::io::BufRead::read_line` on `stdin().lock()` | std. |
| `setvbuf(_IOLBF)` | explicit `flush()` after each printed line in `-t` mode | Rust stdout is line-buffered on TTYs but block-buffered on pipes; the C code forces line buffering, so flush per line to match. |
| `assert()` in tests | `assert!` / `assert_eq!` in `#[cfg(test)]` modules | std, run via `cargo test`. |
| `abort()` on getentropy failure | `panic!` (or `expect`) | Matches "abort on failure" semantics. |

**Chosen dependency set:** `getrandom = "0.3"` (only external crate).
Everything else is std. (If the implementer prefers, `rand` with the
`getrandom` feature is an acceptable substitute, but `getrandom` alone is
smaller and closer to the C code.)

## 3. Target project layout

```
ulidgen/
├── Cargo.toml          # package "ulidgen", edition 2021, bin + lib
├── src/
│   ├── lib.rs          # ulidgen_r() core + #[cfg(test)] tests
│   └── main.rs         # CLI (ulidgen binary)
└── (no tests/ dir needed — tests live in lib.rs, run by `cargo test`)
```

`Cargo.toml` sketch:

```toml
[package]
name = "ulidgen"
version = "0.1.0"
edition = "2021"

[dependencies]
getrandom = "0.3"
```

### `src/lib.rs` — API design

Faithful port of the C API, keeping the stateful-buffer contract:

```rust
/// Generate a ULID into `ulid` (27 bytes: 26 chars + NUL), matching
/// the C `ulidgen_r(char ulid[27])`. The previous ULID left in the
/// buffer is used for the same-millisecond increment path.
pub fn ulidgen_r(ulid: &mut [u8; 27])
```

Implementation notes:
- `const B32: &[u8; 32] = *b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";`
- Timestamp: `SystemTime::now().duration_since(UNIX_EPOCH)` →
  `secs * 1000 + nanos / 1_000_000` as `u64`.
- Timestamp encoding: loop `i` from 9 down to 0, `t /= 32`,
  `ulid[i] = B32[(t % 32) as usize]`; track `same` by comparing against the
  **pre-existing** buffer value (read `ulid[i]` before overwriting).
- Increment path: operate on `ulid[10..26]`; walk `i` from 15 down while
  `buf[i] == b'Z'` setting `b'0'`; on full overflow `thread::sleep(1.234567 ms)`
  and recurse (or loop — recursion depth is bounded by the sleep, keep
  recursion to mirror the C code); bump via `B32` index lookup
  (`B32.iter().position(|&c| c == buf[i])`), successor = `pos + 1`; if the
  char isn't in the alphabet, fall through to re-randomization.
- Random path: `let mut rnd = [0u8; 16]; getrandom(&mut rnd).expect(...)`;
  `buf[i] = B32[rnd[i] as usize % 32]` — **preserve the `% 32` bias** for
  behavioral fidelity.
- Keep `ulid[26] = 0` (NUL terminator) so the buffer stays C-compatible
  and `String::from_utf8_lossy`/`str` conversions in the CLI are clean.

A convenience wrapper for the CLI is fine (e.g. return `String`), but the
`[u8; 27]` API must exist to mirror the source.

### `src/main.rs` — CLI

- Parse `std::env::args().skip(1)`:
  - `-n N` → `n: i64 = N.parse().unwrap_or(abort)` (C uses `atol`;
    `unwrap_or_else(|_| exit(1))` or `expect` is fine).
  - `-t` → tag mode.
  - Unknown flag → print usage to stderr, exit non-zero (C's getopt would
    error similarly; keep it simple).
- Tag mode:
  ```rust
  let stdin = io::stdin();
  let mut stdout = io::stdout();
  let mut line = String::new();
  let mut ulid = [0u8; 27];
  for _ in iter::repeat(()) {
      line.clear();
      match stdin.lock().read_line(&mut line) { Ok(0) => break, Ok(_) => {}, Err(_) => break }
      ulidgen_r(&mut ulid);
      write!(stdout, "{} {}", str::from_utf8(&ulid[..26]).unwrap(), line).unwrap();
      stdout.flush().unwrap();   // mirrors setvbuf(_IOLBF)
  }
  ```
- Generate mode: loop `0..n`, `ulidgen_r(&mut ulid)`, `println!` the 26 chars.
- Exit status: propagate write errors → `std::process::exit(1)` (mirrors
  `exit(!!ferror(stdout))`).

### Tests (in `lib.rs`, `#[cfg(test)] mod tests`)

Port all four C tests, including the currently-disabled structure test
(keep it enabled or commented to mirror the source — recommend enabled,
it passes):

1. `test_ulid_length` — `ulid[..26]` is 26 ASCII chars, `ulid[26] == 0`.
2. `test_ulid_structure` — every char in the Crockford alphabet.
3. `test_ulid_uniqueness` — two consecutive ULIDs differ (reuse one buffer,
   as the C test does with two buffers — either works; use two buffers to
   mirror the C test exactly).
4. `test_ulid_sortability` — generate, `thread::sleep(1.5 ms)`, generate,
   assert `ulid1 < ulid2` lexicographically.

Run with `cargo test`.

## 4. Translation risks

1. **Stateful buffer contract.** The increment path reads the *previous*
   ULID from the caller's buffer. In Rust this is natural (mutable borrow)
   but easy to break if the implementer "improves" the API to return a
   fresh `String` and drops the old value — the same-millisecond uniqueness
   guarantee would silently change. Keep the `&mut [u8; 27]` API.
2. **Biased `% 32` encoding.** `rnd[i] % 32` is not uniform base32. Do not
   replace it with proper base32 encoding; tests and behavior depend on the
   exact alphabet positions (e.g. the `'Z'`→`'0'` carry logic assumes the
   alphabet order).
3. **`same` detection timing.** The C code compares each timestamp char
   against the buffer *before* overwriting it. The Rust port must capture
   the old value per iteration, not compare after the loop.
4. **Overflow recursion + sleep.** The all-`'Z'` case sleeps 1.234567 ms
   and recurses. Keep the sleep duration exact; converting to a loop is
   fine but must re-fetch the timestamp (recursion does this naturally).
5. **Line buffering in `-t` mode.** Rust block-buffers stdout on pipes;
   without per-line `flush()`, piped output ordering vs. other processes
   differs from the C original. Flush after each line.
6. **`getdelim` vs `read_line`.** Both keep the trailing newline and handle
   arbitrarily long lines; equivalent. Watch out for a final line without
   newline — both handle it (read_line returns >0).
7. **Exit status.** C exits 1 only if stdout writes failed. Rust `write!`
   panics on error by default — either let it panic (non-zero exit,
   acceptable) or catch and `exit(1)` to mirror exactly.
8. **`atol` leniency.** C's `atol` silently accepts garbage (→ 0). Rust
   `parse::<i64>()` errors; choose `expect`/`exit(1)` — a deliberate,
   defensible divergence (arguably a fix).
9. **Coverage report.** The source ships `coverage_report.json` (gcov
   output). Not needed for the Rust build; `cargo test` is the test gate.
   Optionally note `cargo-llvm-cov` for equivalent coverage.

## 5. Verification plan

- `cargo build` — compiles lib + bin.
- `cargo test` — all 4 tests pass (length, structure, uniqueness, sortability).
- Smoke: `cargo run -- -n 3` prints 3 distinct 26-char Crockford-ULIDs;
  `echo "hello" | cargo run -- -t` prints `<ULID> hello`.
- Sortability smoke: `cargo run -- -n 5` output is lexicographically
  non-decreasing.
