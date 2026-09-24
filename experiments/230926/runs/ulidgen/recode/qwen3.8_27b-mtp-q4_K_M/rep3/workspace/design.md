# ulidgen — C → Rust Translation Design

## 1. Source project analysis

`ulidgen` is a tiny public-domain CLI tool (by Leah Neukirchen) that generates
ULIDs (Universally Unique Lexicographically Sortable Identifiers) or prefixes
each line of stdin with one.

### Files

| File | Role |
|---|---|
| `src/ulid.h` | Single declaration: `void ulidgen_r(char[27]);` |
| `src/ulid.c` | Core ULID generator `ulidgen_r` |
| `src/ulidgen.c` | `main`: getopt CLI (`-n N`, `-t`), output loop |
| `tests/test.c` | 4 tests (length, structure [commented out], uniqueness, sortability) |
| `Makefile` | Builds `ulid.o`, compiles+runs `test_1`, install rules |

### Core algorithm (`ulidgen_r`)

1. Crockford base32 alphabet: `"0123456789ABCDEFGHJKMNPQRSTVWXYZ"` (no I, L, O, U).
2. Buffer is 27 bytes: 26 chars + NUL. `ulid[26] = 0`.
3. Timestamp: `clock_gettime(CLOCK_REALTIME)` → milliseconds
   (`tv.tv_sec*1000 + tv.tv_nsec/1000000`), a 48-bit value encoded big-endian
   into `ulid[0..10]` via `for (i = 9; i >= 0; i--, t /= 32) ulid[i] = b32[t % 32]`.
4. **Same-millisecond increment path**: the function compares against the
   *previous contents of the caller's buffer* (the `same` flag is set only if
   all 10 timestamp chars already matched). If same:
   - scan `buf[15..0]` (the 16 random chars) from the right; while `buf[i] == 'Z'`
     set `buf[i] = '0'`;
   - if the scan runs past index 0 (`i < 0`): `nanosleep(0, 1234567)` (≈1.23 ms)
     and **recurse** `ulidgen_r(ulid)`;
   - otherwise find `buf[i]` in the alphabet and advance to the next character;
   - if `buf[i]` is not in the alphabet (corrupt buffer), fall through and
     re-randomize.
5. **Random path**: `getentropy(rnd, 16)`; on failure `abort()`. Then
   `buf[i] = b32alphabet[rnd[i] % 32]` for `i in 0..16` (note: 16 bytes →
   16 chars, one byte per char, with a slight modulo bias — preserve as-is).

### CLI (`ulidgen.c`)

- `getopt(argc, argv, "n:t")`: `-n N` (default 1, parsed with `atol`), `-t`.
- `-t` mode: `setvbuf(stdout, 0, _IOLBF, 0)` (line-buffered), then
  `getdelim` loop printing `"%s %s"` (ULID, space, line — line keeps its `\n`).
- Default mode: print `n` ULIDs, one per line (`puts`).
- Exit status: `exit(!!ferror(stdout))` — 0 on success, 1 if a write failed.

### Tests (`tests/test.c`)

- `test_ulid_length`: strlen == 26.
- `test_ulid_structure`: all chars in alphabet (currently commented out in main).
- `test_ulid_uniqueness`: two consecutive ULIDs differ (relies on the
  increment path when generated in the same millisecond).
- `test_ulid_sortability`: after a 1.5 ms `nanosleep`, the second ULID is
  lexicographically greater.

### Build/test

`make test` compiles `tests/test.c src/ulid.c` and runs the binary. No
third-party C dependencies — only libc (`getentropy`, `clock_gettime`,
`nanosleep`, `getopt`, `getdelim`).

## 2. Dependency mapping (C → Rust)

| C feature | Rust counterpart | Notes |
|---|---|---|
| `getentropy(2)` | **`getrandom` crate** (`getrandom::fill`) | Direct idiomatic counterpart. Fallback: `rand` crate. |
| `clock_gettime(CLOCK_REALTIME)` | `std::time::SystemTime::now()` + `Duration::since(UNIX_EPOCH)` | std only; ms = `as_secs()*1000 + as_nanos()/1_000_000`. |
| `nanosleep` | `std::thread::sleep(Duration::from_nanos(1_234_567))` | std only. |
| `getopt` | **`clap` crate** (derive or builder) | Idiomatic CLI parsing; supports `-n N` and `-t`. (Manual `std::env::args` parsing is an acceptable zero-dep alternative.) |
| `getdelim` / `getline` | `std::io::stdin().lock().lines()` | std only. |
| `setvbuf(_IOLBF)` | `std::io::LineWriter` around stdout, or flush per line | std only. |
| `abort()` on entropy failure | `panic!` / `std::process::abort()` | Preserve hard-fail semantics. |
| `exit(!!ferror(stdout))` | check `io::Result` of writes; `std::process::exit(1)` on error | Preserve exit-status contract. |

**Chosen dependency set:** `getrandom` (required, entropy), `clap` (CLI,
idiomatic). Both are stable, widely used crates. If the pipeline prefers
minimal deps, `clap` can be replaced by a ~15-line manual arg parser and
`getrandom` by `rand` — but `getrandom` is the exact counterpart of
`getentropy` and is the better choice.

## 3. Target project structure

```
Cargo.toml
src/
  lib.rs      # pub fn ulidgen_r(buf: &mut [u8; 27])  +  pub fn ulid() -> String
  main.rs     # CLI: -n N / -t, output loops, exit status
tests/
  ulid.rs     # integration tests mirroring tests/test.c
```

`Cargo.toml`:
```toml
[package]
name = "ulidgen"
version = "0.1.0"
edition = "2021"

[dependencies]
getrandom = "0.2"
clap = { version = "4", features = ["derive"] }
```

### `src/lib.rs` — API design

Mirror the C interface for fidelity, plus an ergonomic wrapper:

```rust
pub const B32_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Fill `buf` (26 chars + NUL) with a ULID, mirroring C `ulidgen_r(char[27])`.
/// The caller reuses the same buffer across calls so the same-millisecond
/// increment logic works exactly as in the C original.
pub fn ulidgen_r(buf: &mut [u8; 27]) { ... }

/// Convenience: return a fresh ULID as a String (26 chars).
pub fn ulid() -> String {
    let mut b = [0u8; 27];
    ulidgen_r(&mut b);
    String::from_utf8(b[..26].to_vec()).unwrap()
}
```

Implementation notes (preserve C semantics exactly):

- Timestamp: `SystemTime::now().duration_since(UNIX_EPOCH)` →
  `let t = (d.as_secs() * 1000 + d.as_nanos() / 1_000_000) as u64;`
  encode with `for i in (0..10).rev() { buf[i] = B32[t % 32]; t /= 32; }`.
- `same` detection: compare `buf[0..10]` against the freshly computed
  timestamp chars *before* overwriting (the C code checks
  `ulid[i] != b32alphabet[t % 32]` per position while writing).
- Increment path: iterate `i` from 15 down; while `buf[i] == b'Z'` set
  `b'0'`; if exhausted → `thread::sleep(1_234_567 ns)` and recurse
  `ulidgen_r(buf)`; else advance the char to its successor in the alphabet;
  if the char is not in the alphabet, fall through to re-randomize.
- Random path: `let mut rnd = [0u8; 16]; getrandom::fill(&mut rnd).unwrap_or_else(|_| std::process::abort());`
  then `buf[i] = B32[rnd[i] % 32]` for `i in 0..16`.
- Keep `buf[26] = 0` (NUL) for byte-level compatibility with the C signature.

### `src/main.rs` — CLI

- Parse with clap: `#[arg(short = 'n', default_value_t = 1, value_parser = parse_long)] n: i64`
  and `#[arg(short = 't')] t: bool`. (C used `atol`; accept the same.)
- `-t` mode: wrap stdout in `LineWriter` (line-buffered, like `_IOLBF`);
  for each line from `stdin().lock().lines()`: `ulidgen_r(&mut buf)` then
  `writeln!(out, "{} {}", str, line)` — note C prints `"%s %s"` where the
  line still contains its trailing `\n`; with Rust `lines()` the newline is
  stripped, so use `writeln!` to restore it.
- Default mode: loop `n` times, `println!` each ULID.
- Exit status: propagate write errors → `std::process::exit(1)` (mirrors
  `exit(!!ferror(stdout))`).

### `tests/ulid.rs` — tests (mirror `tests/test.c`)

```rust
use ulidgen::ulid;

#[test] fn ulid_length() { assert_eq!(ulid().len(), 26); }

#[test] fn ulid_structure() { /* all chars in B32_ALPHABET */ }

#[test] fn ulid_uniqueness() {
    let a = ulid(); let b = ulid();
    assert_ne!(a, b);
}

#[test] fn ulid_sortability() {
    let a = ulid();
    std::thread::sleep(std::time::Duration::from_millis(2)); // ≥1.5 ms, use 2 for safety
    let b = ulid();
    assert!(a < b);
}
```

Note: `ulid_uniqueness` and `ulid_sortability` depend on the same-millisecond
increment path, which requires the *same buffer* to be reused. The `ulid()`
helper creates a fresh zeroed buffer each call, so two calls in the same
millisecond would both take the random path and could theoretically collide
(16 random chars — practically impossible, but the C tests rely on the
increment path). To faithfully mirror the C tests, the test helper should
reuse one persistent buffer (e.g., a `thread_local!` static buffer, or expose
`ulidgen_r` and drive it with a shared `[u8; 27]` in the test). Design
decision: tests call `ulidgen_r` on a shared buffer to reproduce C behavior
exactly; `ulid()` remains for convenience.

## 4. Risks and mitigations

1. **Same-buffer state dependency**: the C "increment" logic only works
   because the caller reuses the buffer. In Rust, `ulid()` (fresh buffer)
   changes behavior in the same-millisecond case. Mitigation: keep
   `ulidgen_r(&mut [u8; 27])` as the primary API; main.rs and tests reuse one
   buffer; document the contract.
2. **Modulo bias** (`rnd[i] % 32`): preserve as-is for fidelity; not a
   correctness issue for ULIDs.
3. **Recursion on wraparound**: keep the recursive call (bounded in practice);
   Rust recursion is fine here (depth ~1).
4. **Exit status**: Rust panics exit with 101, not 1. Mitigation: handle IO
   errors explicitly with `exit(1)`; avoid panics on the normal path.
5. **Line buffering**: `LineWriter` flushes on `\n`; ensure final flush and
   error check to mirror `fflush(0); exit(!!ferror(stdout))`.
6. **`-n` parsing**: C `atol` silently accepts garbage (→ 0). clap's
   `value_parser` rejects invalid input with an error message — a behavior
   improvement, acceptable; note it in the summary.
7. **getrandom availability**: on Linux (target) `getrandom(2)` is present;
   the crate falls back to `getentropy`/`/dev/urandom` if needed.
8. **Test flakiness**: the sortability test needs >1 ms between calls; use
   2 ms sleep to be safe on loaded CI.

## 5. Verification

- `cargo test` runs the integration tests in `tests/ulid.rs`.
- Manual smoke test: `cargo run -- -n 3` (3 ULIDs), `echo hi | cargo run -- -t`
  (line prefixed with ULID), `echo $?` = 0.
