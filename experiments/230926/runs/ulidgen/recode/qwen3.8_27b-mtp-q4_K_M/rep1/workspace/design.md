# ulidgen — C → Rust Translation Design

## 1. Source project analysis

**Project:** `ulidgen` — a tiny public-domain CLI tool (by Leah Neukirchen) that
generates ULIDs (Universally Unique Lexicographically Sortable Identifiers) or
prefixes stdin lines with them.

### Files

| File | Role |
|------|------|
| `src/ulid.c` / `src/ulid.h` | Core library: `void ulidgen_r(char ulid[27])` generates one 26-char Crockford-Base32 ULID into a caller-supplied buffer. |
| `src/ulidgen.c` | CLI `main`: `ulidgen [-n N] [-t]` via `getopt`. |
| `tests/test.c` | 4 tests: length == 26, valid alphabet, uniqueness of consecutive ULIDs, lexicographic sortability (with a 1.5 ms `nanosleep` between generations). |
| `Makefile` | Builds `ulid.o`, compiles `test_1` from `tests/test.c` + `src/ulid.c` and runs it; `install` target for binary + man page. |
| `README` | Man-page text (ulidgen(1)). |

### Core algorithm (`src/ulid.c`, `ulidgen_r`)

1. Alphabet: Crockford Base32 `"0123456789ABCDEFGHJKMNPQRSTVWXYZ"` (no I, L, O, U).
2. `clock_gettime(CLOCK_REALTIME)` → millisecond timestamp `t` (µs precision: `tv_sec*1000 + tv_nsec/1000000`).
3. Encode `t` into the first 10 chars (positions 9..0, `t /= 32` each step), tracking
   whether any char changed (`same` flag).
4. If the timestamp part is unchanged from the previous call (the caller reuses the
   same buffer, so the previous ULID is still in it):
   - **Increment the 16-char random part in place** (`buf = ulid + 10`): scan from
     index 15 down; if all are `'Z'`, wrap them to `'0'`, `nanosleep(0, 1234567)`
     (~1.23 ms) and **recurse** into `ulidgen_r`.
   - Otherwise advance `buf[i]` to the next alphabet char (via `strchr` lookup);
     if the char is not in the alphabet (corrupt buffer), fall through to randomize.
5. Otherwise: fill 16 bytes with `getentropy` (abort on failure) and encode each
   byte as `b32alphabet[rnd[i] % 32]` into positions 10..25.
6. `ulid[26] = 0` (NUL-terminated C string).

Key subtlety: the function is *stateless* — it relies on the caller passing back the
same buffer containing the previous ULID so it can detect "same millisecond" and
increment. The CLI (`src/ulidgen.c`) does exactly this with one `char ulid[27]`.

### CLI behavior (`src/ulidgen.c`)

- `getopt(argc, argv, "n:t")`: `-n N` (default 1, parsed with `atol`), `-t` flag.
- `-t` mode: `setvbuf(stdout, 0, _IOLBF, 0)` (line-buffered), `getdelim` loop over
  stdin, prints `"%s %s"` (ULID, line — line keeps its trailing newline).
- `-n` mode: prints N ULIDs, one per line (`puts`).
- Exit status: `exit(!!ferror(stdout))` — nonzero if a write error occurred.

### Tests (`tests/test.c`)

- `test_ulid_length`: `strlen(ulid) == 26`.
- `test_ulid_structure` (commented out in `main`): all chars in the Crockford alphabet.
- `test_ulid_uniqueness`: two consecutive ULIDs differ.
- `test_ulid_sortability`: ULID1 < ULID2 (strcmp) after a 1.5 ms sleep.

### Build/test setup

Plain Makefile + `gcc`; tests are a separate C file compiled with the library and
executed. **No third-party C dependencies** — only libc (`getopt`, `getentropy`,
`clock_gettime`, `nanosleep`, `getdelim`).

## 2. Dependency mapping (C → Rust)

The C project has **zero third-party dependencies** (libc only). The Rust
translation keeps the dependency surface minimal:

| C facility | Rust counterpart | Notes |
|------------|------------------|-------|
| `getentropy(2)` (libc) | **`getrandom` crate** (`getrandom::fill`) | Rust std has no entropy API; `getrandom` is the idiomatic, thin wrapper over the same syscall. Alternative: `rand::thread_rng()`. |
| `getopt` (libc) | std `env::args` manual loop (chosen) or `clap` | Manual parsing keeps the project dependency-light like the original; `clap` is the idiomatic alternative. |
| `clock_gettime(CLOCK_REALTIME)` | `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` | std, no dep. |
| `nanosleep` | `std::thread::sleep(Duration)` | std. |
| `getdelim` / `setvbuf` | `std::io::BufRead::lines()` on `stdin.lock()` | std; Rust stdout is line-buffered on TTYs by default, and we flush explicitly. |
| `assert.h` | `assert!` / `#[test]` | std. |
| `atol` | `str::parse::<i64>()` | std. |

**Chosen dependency set:** `getrandom` (v0.2, `fill` API) — the only external
crate. Everything else from `std`. (If the implementer prefers, `clap` v4 can
replace manual arg parsing; it is optional and not required for parity.)

## 3. Target project structure

```
ulidgen/
├── Cargo.toml          # package "ulidgen", edition 2021, bin + lib
├── src/
│   ├── lib.rs          # core ULID generation (port of src/ulid.c) + unit tests
│   └── main.rs         # CLI (port of src/ulidgen.c)
└── tests/
    └── ulid.rs         # integration tests (port of tests/test.c)
```

### `Cargo.toml`

```toml
[package]
name = "ulidgen"
version = "0.1.0"
edition = "2021"

[dependencies]
getrandom = "0.2"
```

### `src/lib.rs` — port of `ulid.c`

Preserve the C API shape so the "same buffer" statefulness is explicit:

```rust
pub const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Port of C `ulidgen_r(char ulid[27])`.
/// `ulid` must hold the previously generated ULID (26 bytes) or be zeroed;
/// on return it holds a new 26-byte ULID.
pub fn ulidgen_r(ulid: &mut [u8; 27]) { ... }

/// Convenience: generate a fresh ULID as a String (no prior state).
pub fn ulid() -> String { let mut buf = [0u8; 27]; ulidgen_r(&mut buf); String::from_utf8(buf[..26].to_vec()).unwrap() }
```

Implementation notes (mirror the C logic 1:1):

- Timestamp ms: `SystemTime::now().duration_since(UNIX_EPOCH)` →
  `secs * 1000 + nanos / 1_000_000` as `u64`.
- Encode 10 chars: loop `i` 9..=0, `t /= 32`, set `ulid[i] = B32_ALPHABET[(t % 32) as usize]`,
  track `same` (unchanged from previous content).
- Same-ms branch: `buf = &mut ulid[10..26]`; scan `i` from 15 down while `buf[i] == b'Z'`
  setting `b'0'`; if all wrapped → `thread::sleep(Duration::from_nanos(1_234_567))` and
  recurse `ulidgen_r(ulid)` (keep recursion, as in C).
- Else advance: find current char's index in the alphabet (linear scan, like `strchr`);
  if found → `buf[i] = B32_ALPHABET[(idx + 1) % 32]` and return; if not found → fall
  through to randomize.
- Random branch: `let mut rnd = [0u8; 16]; getrandom::fill(&mut rnd).expect("getentropy failed");`
  then `buf[i] = B32_ALPHABET[rnd[i] as usize % 32]` for `i` in 0..16.
- `ulid[26]` stays `0` (the 27th byte is a sentinel, mirroring the C NUL terminator;
  the public string is always `&buf[..26]`).

Unit tests in `lib.rs` (`#[cfg(test)]`): length, alphabet validity, uniqueness,
sortability (with `thread::sleep(Duration::from_millis(2))` to be safe).

### `src/main.rs` — port of `ulidgen.c`

- Parse `std::env::args().skip(1)`:
  - `-n N` → `n: i64 = N.parse().unwrap_or(1)` (C used `atol`; accept the same leniency).
  - `-t` → `tflag = true`.
  - Unknown flag → print usage to stderr, exit 1 (C's getopt would error similarly).
- `-t` mode: `let stdin = io::stdin(); let mut lines = stdin.lock().lines();`
  for each `Ok(line)` → `ulidgen_r(&mut buf); write!(stdout, "{} {}", ulid_str, line)?;`
  (line from `lines()` has no trailing newline, so append `"\n"` — C's `getdelim`
  kept it; net output is identical).
- `-n` mode: loop `0..n`, `println!("{}", ulid_str)`.
- Exit status: propagate write errors — `fn main() -> io::Result<()>` (or catch and
  `process::exit(1)` on error), matching `exit(!!ferror(stdout))`.
- Flush stdout at the end (Rust flushes on drop; explicit `flush()` for parity).

### `tests/ulid.rs` — port of `tests/test.c`

Integration tests using the public API:

```rust
use ulidgen::ulid;

#[test] fn ulid_length() { assert_eq!(ulid().len(), 26); }
#[test] fn ulid_structure() { /* all chars in "0123456789ABCDEFGHJKMNPQRSTVWXYZ" */ }
#[test] fn ulid_uniqueness() { assert_ne!(ulid(), ulid()); }
#[test] fn ulid_sortability() {
    let a = ulid();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let b = ulid();
    assert!(a < b);
}
```

Run with `cargo test` (unit + integration).

## 4. Risks and mitigations

1. **Statefulness via caller buffer.** The C function is stateless only because the
   caller reuses the buffer. In Rust, `&mut [u8; 27]` preserves this exactly; the
   `ulid()` convenience wrapper starts from a zeroed buffer (first call always
   randomizes — same as C's zero-initialized `char ulid[27] = {0}`).
   *Mitigation:* keep both APIs; document the buffer contract.
2. **`getentropy` not in std.** *Mitigation:* `getrandom` crate (same syscall,
   `fill` API); `expect`/abort on failure mirrors C's `abort()`.
3. **Recursion on all-'Z' wrap.** Rare (needs 16 consecutive same-ms generations
   with a fully-wrapped random part); keep the recursive call as in C — depth is
   bounded in practice by the 1.23 ms sleep.
4. **Sortability test flakiness.** Depends on a >1 ms wall-clock gap; the C test
   sleeps 1.5 ms. *Mitigation:* sleep 2 ms in the Rust test; the algorithm itself
   guarantees ordering within the same ms via in-place increment.
5. **`-t` mode newline handling.** `getdelim` keeps `\n`; Rust `lines()` strips it.
   *Mitigation:* explicitly append `"\n"` when printing, so output bytes are
   identical to the C tool.
6. **Exit code on write error.** *Mitigation:* `main() -> io::Result<()>` gives
   nonzero exit on I/O failure, matching `exit(!!ferror(stdout))`.
7. **`atol` leniency vs `parse`.** C `atol("3abc")` → 3; Rust `parse` fails.
   *Mitigation:* accept either — use `parse().unwrap_or(1)`; edge-case divergence
   is acceptable and arguably better.
8. **Crockford alphabet.** Must be exactly `0123456789ABCDEFGHJKMNPQRSTVWXYZ`
   (32 chars, no I/L/O/U) — a typo breaks validity tests. *Mitigation:* unit test
   asserts alphabet length 32 and membership checks.

## 5. Verification plan

- `cargo build` — compiles lib + bin.
- `cargo test` — unit tests (lib.rs) + integration tests (tests/ulid.rs).
- Smoke: `cargo run -- -n 3` prints 3 distinct 26-char ULIDs;
  `echo hello | cargo run -- -t` prints `<ULID> hello`.
