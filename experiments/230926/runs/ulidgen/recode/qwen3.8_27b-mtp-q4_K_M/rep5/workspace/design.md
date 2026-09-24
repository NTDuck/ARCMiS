# ulidgen — C → Rust Translation Design

## 1. Source project analysis

**Project:** `ulidgen` — a tiny public-domain CLI tool (by Leah Neukirchen) that
generates ULIDs (Universally Unique Lexicographically Sortable Identifiers) or
prefixes stdin lines with them.

### File inventory

| File | Role |
|---|---|
| `Makefile` | Builds `ulid.o`, compiles `tests/test.c` + `src/ulid.c` into `test_1`, runs it; install/README targets. Coverage flags (`-fprofile-arcs -ftest-coverage`). |
| `README` | Rendered man page (usage: `ulidgen [-n N] [-t]`). |
| `src/ulid.h` | Single declaration: `void ulidgen_r(char[27]);` |
| `src/ulid.c` | Core ULID generation (`ulidgen_r`). |
| `src/ulidgen.c` | CLI `main`: getopt parsing, `-n N` / `-t` modes. |
| `tests/test.c` | 4 tests: length, structure (commented out in `main`), uniqueness, sortability. |
| `coverage_report.json` | Per-file line coverage snapshot (71.79% / 100% / 59.26% / 67.65%). |

### Core algorithm (`src/ulid.c`, `ulidgen_r(char ulid[27])`)

1. Crockford Base32 alphabet: `"0123456789ABCDEFGHJKMNPQRSTVWXYZ"` (32 chars, no I/L/O/U).
2. Layout: `ulid[0..10)` = 10-char timestamp, `ulid[10..26)` = 16-char random part, `ulid[26] = '\0'`.
3. Timestamp: `clock_gettime(CLOCK_REALTIME)` → `tv.tv_sec*1000 + tv.tv_nsec/1000000` (ms since epoch),
   encoded big-endian into 10 base32 digits (`for i in 9..=0: digit = t % 32; t /= 32`).
4. **Same-millisecond handling:** if all 10 timestamp digits are unchanged from the previous call
   (state lives in the caller's buffer, `same` flag), the random part is *incremented in place*:
   - carry from index 15 down: while `buf[i] == 'Z'` set `'0'` and continue;
   - if carry falls off (`i < 0`): `nanosleep(0, 1234567)` (≈1.23 ms) and **recurse**;
   - otherwise advance `buf[i]` to the next alphabet character;
   - if `buf[i]` is not in the alphabet (corrupted), fall through to full re-randomization.
5. Fresh random part: 16 bytes from `getentropy(2)` (on failure: `abort()`), each byte mapped
   `b32alphabet[rnd[i] % 32]` (note: modulo bias is present in the original — preserve for fidelity).

### CLI (`src/ulidgen.c`)

- `getopt(argc, argv, "n:t")`: `-n N` (default 1, `atol`), `-t` (tag mode).
- Tag mode: `setvbuf(stdout, 0, _IOLBF, 0)` (line-buffered), `getdelim` loop, prints `"%s %s"` (ULID, space, line incl. newline).
- Generate mode: loop `n` times, `puts(ulid)`.
- Exit status: `exit(!!ferror(stdout))` → 0 on success, 1 on write error.

### Tests (`tests/test.c`)

1. `test_ulid_length` — strlen == 26.
2. `test_ulid_structure` — all chars in alphabet (defined but **not called** from `main`).
3. `test_ulid_uniqueness` — two consecutive ULIDs differ.
4. `test_ulid_sortability` — after `nanosleep(1.5 ms)`, second ULID > first lexicographically.

### Source dependencies

**None beyond libc.** System calls used: `clock_gettime`, `getentropy`, `nanosleep`,
`getopt`, `getdelim`, `setvbuf`, `atol`, `abort`. No third-party C libraries.

## 2. Dependency mapping (C → Rust)

| C dependency / syscall | Rust counterpart | Notes |
|---|---|---|
| `getentropy(2)` | **`getrandom` crate** (`getrandom::fill`) | Direct idiomatic counterpart; `rand` is an alternative but `getrandom` is the minimal, precise match. |
| `clock_gettime(CLOCK_REALTIME)` | `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` | std, no dep. |
| `nanosleep` | `std::thread::sleep(Duration)` | std. |
| `getopt` | `std::env::args` manual parse (or `clap` if a heavier CLI is desired) | Keep zero-dep: the option set is trivial (`-n N`, `-t`); manual parsing mirrors the C behavior exactly. |
| `setvbuf(_IOLBF)` | not needed | Rust `Stdout` is line-buffered by default when attached to a terminal. |
| `getdelim` | `std::io::BufRead::read_line` | std. |
| `abort()` on entropy failure | `std::process::abort()` | same semantics. |
| `assert` in tests | `assert!` / `assert_eq!` | std. |

**Resulting third-party dependency list: `getrandom = "0.2"` only.**

## 3. Target project structure

```
ulidgen/
├── Cargo.toml
├── src/
│   ├── lib.rs      # pub fn ulidgen_r(ulid: &mut [u8; 27])  (port of src/ulid.c)
│   └── main.rs     # CLI: -n N / -t modes (port of src/ulidgen.c)
└── tests/
    └── test.rs     # integration tests (port of tests/test.c)
```

### Cargo.toml

```toml
[package]
name = "ulidgen"
version = "0.1.0"
edition = "2021"
description = "generate or tag lines with ULID"
license = "Unlicense"

[dependencies]
getrandom = "0.2"
```

### `src/lib.rs` — API design

Mirror the C signature (in-place buffer, NUL-terminated) so the translation is 1:1:

```rust
pub const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into `ulid` (26 chars + NUL at index 26), like C `ulidgen_r`.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    let buf = unsafe { ulid.get_unchecked_mut(10..26) }; // or slice via split_at_mut
    let mut same = true;
    ulid[26] = 0;

    let ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
        .as_secs() * 1000 + duration.subsec_nanos() as u64 / 1_000_000;

    let mut t = ms;
    for i in (0..10).rev() {
        let d = B32_ALPHABET[(t % 32) as usize];
        if ulid[i] != d { ulid[i] = d; same = false; }
        t /= 32;
    }

    if same {
        // increment random part in place (carry from index 15)
        let mut i = 15;
        while i < 16 && buf[i] == b'Z' { buf[i] = b'0'; i -= 1; }
        if i < 0 {
            thread::sleep(Duration::from_nanos(1_234_567));
            ulidgen_r(ulid);
            return;
        }
        if let Some(pos) = B32_ALPHABET.iter().position(|&c| c == buf[i]) {
            buf[i] = B32_ALPHABET[pos + 1];
            return;
        }
        // else: invalid char → re-randomize below
    }

    let mut rnd = [0u8; 16];
    if getrandom::fill(&mut rnd).is_err() { std::process::abort(); }
    for (i, b) in rnd.iter().enumerate() {
        buf[i] = B32_ALPHABET[(*b % 32) as usize];
    }
}
```

Key fidelity points:
- `same` detection compares against the *caller's* previous buffer contents — same as C,
  so the caller must reuse the same buffer (as `main` does).
- Preserve the `rnd[i] % 32` modulo mapping (bias included) for behavioral parity.
- Preserve recursion on carry-off and the 1,234,567 ns sleep.
- Use `split_at_mut` (safe) instead of raw pointer arithmetic for the `buf` slice.

### `src/main.rs` — CLI design

```rust
fn main() {
    let mut ulid = [0u8; 27];
    let mut n: i64 = 1;
    let mut tflag = false;

    // manual parse of: -n N  and  -t   (mirrors getopt "n:t")
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-n" => n = args.next().expect("-n requires an argument").parse().unwrap_or(1),
            "-t" => tflag = true,
            _ => { eprintln!("usage: ulidgen [-n N] [-t]"); std::process::exit(1); }
        }
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        let mut lines = stdin.lock().lines();
        while let Ok(line) = lines.next_line() {
            ulidgen_r(&mut ulid);
            let _ = write!(out, "{} {}", str::from_utf8(&ulid[..26]).unwrap(), line);
        }
    } else {
        for _ in 0..n.max(0) {
            ulidgen_r(&mut ulid);
            let _ = writeln!(out, "{}", str::from_utf8(&ulid[..26]).unwrap());
        }
    }

    let ok = out.flush().is_ok();
    std::process::exit(if ok { 0 } else { 1 });
}
```

Notes:
- `lines()` strips the trailing newline; re-append `"\n"` when writing to preserve
  `"%s %s"` output exactly (each input line keeps its newline).
- Exit status mirrors `exit(!!ferror(stdout))`: 0 on success, 1 on write/flush error.
- Line buffering: Rust's `Stdout` is already line-buffered on TTYs; no `setvbuf` equivalent needed.

### `tests/test.rs` — test port

```rust
use ulidgen::ulidgen_r;

fn is_valid_ulid(ulid: &str) -> bool {
    const A: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    ulid.len() == 26 && ulid.bytes().all(|c| A.as_bytes().contains(&c))
}

#[test] fn ulid_length() { let mut u = [0u8; 27]; ulidgen_r(&mut u);
    assert_eq!(std::str::from_utf8(&u[..26]).unwrap().len(), 26); }

#[test] fn ulid_structure() { let mut u = [0u8; 27]; ulidgen_r(&mut u);
    assert!(is_valid_ulid(std::str::from_utf8(&u[..26]).unwrap())); }

#[test] fn ulid_uniqueness() { let mut a = [0u8; 27]; let mut b = [0u8; 27];
    ulidgen_r(&mut a); ulidgen_r(&mut b);
    assert_ne!(std::str::from_utf8(&a[..26]).unwrap(), std::str::from_utf8(&b[..26]).unwrap()); }

#[test] fn ulid_sortability() { let mut a = [0u8; 27]; let mut b = [0u8; 27];
    ulidgen_r(&mut a);
    std::thread::sleep(std::time::Duration::from_micros(1500));
    ulidgen_r(&mut b);
    assert!(std::str::from_utf8(&a[..26]).unwrap() < std::str::from_utf8(&b[..26]).unwrap()); }
```

- The C `main` never called `test_ulid_structure`; in Rust all four are real `#[test]`s
  (strictly more coverage than the C suite — fine, and matches the intent).
- Test command: `cargo test` (runs the integration tests in `tests/`).

## 4. Translation risks

1. **Stateful "same" flag semantics.** The C function relies on the caller reusing the same
   buffer to detect same-millisecond calls. Rust keeps this exactly (caller-owned buffer), but
   it is a footgun: a fresh zeroed buffer always takes the random path. Document it in `lib.rs`.
2. **Modulo bias (`rnd[i] % 32`).** Intentionally preserved for behavioral parity; do not
   "improve" it to rejection sampling or outputs will diverge from the C tool.
3. **Recursion depth on carry-off.** Pathological (16 consecutive 'Z's) triggers sleep + recurse;
   identical to C, bounded in practice. Keep as-is.
4. **Sortability test flakiness.** Depends on a 1.5 ms sleep actually crossing a millisecond
   boundary; same flakiness as the C test. Acceptable — do not add retries that change semantics.
5. **`getentropy` vs `getrandom` crate.** The crate may use `getrandom(2)` on Linux (same
   syscall) or a fallback; failure handling maps to `std::process::abort()`.
6. **Output byte-exactness in `-t` mode.** `read_line`/`lines()` strip newlines; must re-append
   `"\n"` to match `printf("%s %s", ulid, line)` where `line` includes its newline. Edge case:
   a final line without trailing newline — C prints it without a newline; Rust `lines()`
   still yields it, so re-appending `"\n"` would *add* one. To be byte-exact, prefer
   `BufRead::read_line` (keeps the newline when present) instead of `lines()`.
7. **`-n` parsing.** C uses `atol` (lenient, no error on garbage). Rust `parse::<i64>()` is
   stricter; fall back to 1 on parse error to stay close, and clamp negative `n` to 0 loops
   (C's `long i < n` loop simply doesn't run for n ≤ 0).

## 5. Build & test

- Build: `cargo build` (binary `ulidgen`).
- Test: `cargo test` — runs `tests/test.rs` (4 tests).
- No Makefile needed; `cargo` replaces it. Install via `cargo install --path .` if desired.
