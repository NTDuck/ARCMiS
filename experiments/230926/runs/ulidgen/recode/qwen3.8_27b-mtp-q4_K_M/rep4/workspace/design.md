# ulidgen — C → Rust Translation Design

## 1. Source project research

### 1.1 What the project is

`ulidgen` is a tiny public-domain CLI utility (by Leah Neukirchen, Void Linux) that
generates ULIDs — Universally Unique Lexicographically Sortable Identifiers — or
prefixes each line of standard input with a ULID.

### 1.2 File inventory

| File | Role |
|------|------|
| `Makefile` | Builds `ulid.o` (object only), compiles+runs `tests/test.c` as `test_1`, install/man targets. Coverage flags (`-fprofile-arcs -ftest-coverage`). |
| `src/ulid.h` | Single declaration: `void ulidgen_r(char[27]);` |
| `src/ulid.c` | Core ULID generator (the only library logic). |
| `src/ulidgen.c` | `main()`: option parsing (`-n N`, `-t`), stdin tagging, output. |
| `tests/test.c` | 4 tests: length, structure (commented out), uniqueness, sortability. |
| `README` | Rendered man page (usage, examples, license). |
| `coverage_report.json` | Historical coverage data — not part of the build, ignore. |

### 1.3 Public interface

Exactly one function:

```c
void ulidgen_r(char ulid[27]);   // fills 26 chars + NUL into caller's buffer
```

The function is *stateful via the caller's buffer*: it compares the timestamp
prefix it is about to write against the buffer's existing contents to decide
whether the millisecond is the same as the previous call.

### 1.4 Core algorithm (`src/ulid.c`, `ulidgen_r`)

1. Alphabet: Crockford Base32 `"0123456789ABCDEFGHJKMNPQRSTVWXYZ"` (32 chars,
   no `I L O U`).
2. `ulid[26] = 0` (NUL terminator; Rust equivalent: no terminator needed).
3. `clock_gettime(CLOCK_REALTIME)` → `t = tv.tv_sec*1000 + tv.tv_nsec/1000000`
   (milliseconds since epoch, `uint64_t`).
4. Timestamp encoding: `for (i = 9; i >= 0; i--, t /= 32)`
   `ulid[i] = b32alphabet[t % 32]`. The first 10 chars encode the ms timestamp
   little-endian-ish (least significant 5-bit group at index 9).
   `same` is set to 0 if any of these chars differs from the buffer's prior
   content (i.e. the millisecond changed, or the buffer was fresh/zeroed).
5. If `same` (same millisecond as the previous call in the *same* buffer):
   increment the 16-char random part in place:
   - scan `buf[15]` down to `buf[0]` (where `buf = ulid + 10`);
   - wrap `'Z'` → `'0'` and continue;
   - if all 16 were `'Z'` (`i < 0`): `nanosleep(0, 1234567)` (~1.23 ms) and
     **recurse** `ulidgen_r(ulid)`;
   - otherwise advance the char to its successor in the alphabet
     (`strchr(b32alphabet, buf[i])` then `*(s+1)`);
   - if the char is not in the alphabet (corrupt buffer), fall through to
     full re-randomization.
6. Otherwise (new millisecond / fresh buffer): `getentropy(rnd, 16)`;
   `abort()` on failure; then `buf[i] = b32alphabet[rnd[i] % 32]` for 16 bytes.

### 1.5 CLI behavior (`src/ulidgen.c`)

- `getopt(argc, argv, "n:t")`:
  - `-n N` → count, `atol(optarg)`, default 1;
  - `-t` → tag mode.
- Tag mode: `setvbuf(stdout, 0, _IOLBF, 0)` (line buffering), then
  `getdelim` loop printing `"%s %s"` (ULID, space, line including its newline).
- Count mode: loop `n` times, `puts(ulid)`.
- `fflush(0); exit(!!ferror(stdout));` → exit 0 on success, 1 if stdout write
  failed.

### 1.6 Tests (`tests/test.c`)

- `test_ulid_length`: `strlen(ulid) == 26`.
- `test_ulid_structure`: 26 chars, all in the alphabet (currently commented
  out in `main`, but present).
- `test_ulid_uniqueness`: two consecutive calls (separate buffers) differ.
- `test_ulid_sortability`: call, `nanosleep(1.5 ms)`, call; first < second
  lexicographically.

### 1.7 Build/test setup

Plain `gcc` + `Makefile`; tests are a standalone C program run directly.
**No third-party dependencies** — only libc (`getopt`, `getentropy`,
`clock_gettime`, `nanosleep`, `getdelim`).

## 2. Third-party library analysis

The C project has **zero third-party dependencies**; it uses libc primitives.
Mapping each to idiomatic Rust:

| C / libc feature | Rust counterpart | Notes |
|------------------|------------------|-------|
| `getentropy(buf, n)` | **`getrandom` crate** (v0.3) | Only true third-party dep. `getrandom::getrandom(&mut buf)`. Fails with `Err` instead of aborting. |
| `clock_gettime(CLOCK_REALTIME)` | `std::time::{SystemTime, UNIX_EPOCH}` | `SystemTime::now().duration_since(UNIX_EPOCH).as_millis()` → `u64`. No dep. |
| `nanosleep(0, 1234567)` | `std::thread::sleep(Duration::from_nanos(1_234_567))` | No dep. |
| `getopt("n:t")` | manual `std::env::args` loop (chosen) or `clap` | Manual parsing keeps the tool dependency-light and matches the trivial `-n`/`-t` surface. `clap` is the idiomatic heavy alternative. |
| `getdelim` / `setvbuf(_IOLBF)` | `std::io::BufRead::lines` + explicit `flush()` per line | No dep. |
| `atol` | `str::parse::<i64>()` with error handling | No dep. |
| `assert()` in tests | `assert!` / `assert_eq!` in `#[test]` | No dep. |

**Chosen dependency set:** `getrandom = "0.3"` only. Everything else is `std`.
(`clap` is listed as the idiomatic `getopt` counterpart but is *not* required;
manual parsing is used to mirror the minimal C tool.)

## 3. Target project design (Rust)

### 3.1 Crate layout

```
ulidgen/
├── Cargo.toml
├── src/
│   ├── lib.rs        # public API: ulidgen_r()  (mirrors src/ulid.c + ulid.h)
│   └── main.rs       # CLI: -n / -t              (mirrors src/ulidgen.c)
└── tests/
    └── test.rs       # integration tests         (mirrors tests/test.c)
```

`Cargo.toml`:
```toml
[package]
name = "ulidgen"
version = "0.1.0"
edition = "2021"

[dependencies]
getrandom = "0.3"
```

`cargo test` is the test command (per task).

### 3.2 `src/lib.rs` — core generator

Preserve the C signature's *semantics* (caller reuses a buffer; the function
detects "same millisecond" by comparing against prior contents) while being
Rust-idiomatic. Public API:

```rust
/// Crockford Base32 alphabet (no I L O U).
pub const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into `ulid`.
///
/// `ulid` must hold 27 bytes; on success the first 26 bytes are the ULID and
/// `ulid[26]` is set to 0 (NUL), mirroring the C `char[27]` contract so the
/// function can be called repeatedly on the same buffer (same-ms increment).
pub fn ulidgen_r(ulid: &mut [u8; 27]);
```

Implementation steps (faithful port):
1. `ulid[26] = 0`.
2. `let t: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;`
3. Timestamp loop:
   ```rust
   let mut same = true;
   let mut t = t;
   for i in (0..10).rev() {
       let c = B32_ALPHABET[(t % 32) as usize];
       if ulid[i] != c { ulid[i] = c; same = false; }
       t /= 32;
   }
   ```
4. If `same`: increment the random part in place over `ulid[10..26]`:
   - walk `i` from 15 down to 0 (index into the 16-char random region);
   - if `ulid[10+i] == b'Z'` set it to `b'0'` and continue;
   - else if the byte is in the alphabet, set it to the alphabet successor and
     return;
   - if all were `Z`: `thread::sleep(Duration::from_nanos(1_234_567))` and
     **loop** (use a `loop`/retry instead of C's recursion to avoid unbounded
     stack growth);
   - if a byte is not in the alphabet, fall through to re-randomization.
5. Else: `let mut rnd = [0u8; 16]; getrandom::getrandom(&mut rnd).expect("getentropy failed");`
   then `ulid[10+i] = B32_ALPHABET[rnd[i] % 32]` for `i in 0..16`.

> Note: `getrandom` failure → `expect`/panic mirrors C's `abort()`.

### 3.3 `src/main.rs` — CLI

```rust
use std::io::{self, Write, BufRead};
use std::env;

fn main() -> std::io::Result<()> {
    let mut n: i64 = 1;
    let mut tflag = false;
    // manual getopt("n:t") equivalent
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-n" => { n = args.next().expect("-n requires a value").parse().expect("invalid -n"); }
            "-t" => { tflag = true; }
            _ => { eprintln!("usage: ulidgen [-n N] [-t]"); std::process::exit(2); }
        }
    }

    let mut ulid = [0u8; 27];
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = line?;
            ulidgen::ulidgen_r(&mut ulid);
            out.write_all(ulid.as_slice())?;   // 26 chars (skip NUL at [26])
            out.write_all(b" ")?;
            out.write_all(line.as_bytes())?;
            out.write_all(b"\n")?;
            out.flush()?;                        // line-buffered behavior
        }
    } else {
        for _ in 0..n {
            ulidgen::ulidgen_r(&mut ulid);
            out.write_all(ulid.as_slice())?;
            out.write_all(b"\n")?;
        }
    }
    out.flush()?;
    Ok(())
}
```

Exit status: `main -> io::Result` yields exit 0 on success and non-zero on a
write error, matching `exit(!!ferror(stdout))`.

### 3.4 `tests/test.rs` — integration tests

Mirror `tests/test.c` using the public `ulidgen::ulidgen_r`:

```rust
use ulidgen::ulidgen_r;

fn is_valid_ulid(ulid: &[u8]) -> bool {
    const A: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    ulid.len() == 26 && ulid.iter().all(|c| A.contains(c))
}

#[test]
fn ulid_length() { let mut u=[0u8;27]; ulidgen_r(&mut u); assert_eq!(u[26],0); assert!(is_valid_ulid(&u[..26])); }

#[test]
fn ulid_structure() { let mut u=[0u8;27]; ulidgen_r(&mut u); assert!(is_valid_ulid(&u[..26])); }

#[test]
fn ulid_uniqueness() {
    let mut a=[0u8;27]; let mut b=[0u8;27];
    ulidgen_r(&mut a); ulidgen_r(&mut b);
    assert_ne!(&a[..26], &b[..26]);
}

#[test]
fn ulid_sortability() {
    let mut a=[0u8;27]; let mut b=[0u8;27];
    ulidgen_r(&mut a);
    std::thread::sleep(std::time::Duration::from_millis(2)); // >1.5ms, safe margin
    ulidgen_r(&mut b);
    assert!(std::str::from_utf8(&a[..26]).unwrap() < std::str::from_utf8(&b[..26]).unwrap());
}
```

(Use a 2 ms sleep for a slightly safer margin than the C 1.5 ms while keeping
the same intent.)

## 4. Translation risks & mitigations

1. **Buffer-reuse / "same" statefulness.** The C function is stateful only
   because the caller reuses the buffer. The Rust port keeps the identical
   `&mut [u8; 27]` contract, so both the CLI (single reused buffer) and the
   tests (fresh buffers) behave exactly as in C. *Mitigation:* keep the
   signature and the compare-against-prior-contents logic verbatim.
2. **Recursion on all-`Z` overflow.** C recurses after a sleep; a pathological
   hot loop could stack-overflow. *Mitigation:* replace recursion with a
   `loop` + `thread::sleep` retry in Rust (safer, same observable behavior).
3. **`getentropy` failure.** C calls `abort()`. *Mitigation:* `getrandom`
   returns `Result`; use `.expect(...)` to preserve the "never continue with
   weak randomness" guarantee (panic ≈ abort for this tool).
4. **NUL terminator vs Rust strings.** C stores 26 chars + NUL in `char[27]`.
   *Mitigation:* keep the 27-byte buffer and NUL at `[26]` for API fidelity,
   but only emit the first 26 bytes to stdout (avoid printing a stray NUL).
5. **`atol` vs `parse`.** `atol` silently returns 0 on garbage; Rust `parse`
   errors. *Mitigation:* treat invalid `-n` as a usage error (exit 2) — a
   strict improvement, no functional regression for valid input.
6. **Line buffering.** C uses `setvbuf(_IOLBF)`. *Mitigation:* explicit
   `flush()` after each tagged line in `-t` mode.
7. **Sortability test flakiness.** Depends on the 1.5 ms sleep crossing a
   millisecond boundary. *Mitigation:* bump to 2 ms in the Rust test for a
   safer margin; logic unchanged.
8. **`getrandom` version API.** v0.3 changed the crate layout vs v0.2.
   *Mitigation:* pin `getrandom = "0.3"` and use `getrandom::getrandom(&mut buf)`.

## 5. Verification plan

- `cargo build` — compiles lib + bin.
- `cargo test` — runs the 4 ported tests (length, structure, uniqueness,
  sortability).
- Manual smoke: `cargo run -- -n 3` (3 distinct ULIDs), and
  `printf 'a\nb\n' | cargo run -- -t` (each line prefixed with a ULID).
