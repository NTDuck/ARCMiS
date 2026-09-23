# ulidgen — C → Rust Translation Design

## 1. Source project overview

`ulidgen` is a tiny, public-domain C utility (by Leah Neukirchen) that generates
ULIDs (Universally Unique Lexicographically Sortable Identifiers) and can tag
stdin lines with them.

### File inventory

| File | Role |
|------|------|
| `src/ulid.c` | Core: `ulidgen_r(char ulid[27])` — fills a 27-byte buffer (26 chars + NUL) with one ULID. |
| `src/ulid.h` | Prototype: `void ulidgen_r(char[27]);` |
| `src/ulidgen.c` | CLI `main`: `-n N` (generate N, default 1) and `-t` (prefix each stdin line). |
| `tests/test.c` | 4 tests: length, structure, uniqueness, sortability. |
| `Makefile` | Builds `ulid.o`, compiles & runs `test_1`, install/man targets. |
| `README` | Rendered man page. |
| `coverage_report.json` | Per-file line-coverage snapshot (informational only). |

### Build / test setup (source)
- `make test` → `gcc $(CFLAGS) -o test_1 tests/test.c src/ulid.c && ./test_1`.
- No external libraries: only libc (`stdint`, `stdlib`, `string`, `time`, `unistd`).
- Entropy source: `getentropy(2)` (Linux).
- Target test command for the translation: **`cargo test`**.

## 2. Core algorithm (faithful reading of `src/ulid.c`)

A ULID is 26 Crockford-Base32 chars: **10 timestamp chars + 16 random chars**.
Alphabet: `0123456789ABCDEFGHJKMNPQRSTVWXYZ` (32 symbols, no I/L/O/U).

The function is **stateful via buffer reuse**: the caller passes the *same* buffer
each call, and the code compares the new timestamp against the *previous* contents
to decide whether it is the same millisecond.

Steps:
1. `buf = ulid + 10` (the 16-char random region, `buf[0]=ulid[10]` … `buf[15]=ulid[25]`).
2. `same = 1`; `ulid[26] = 0`.
3. `t = tv.tv_sec*1000 + tv.tv_nsec/1000000` (ms since epoch).
4. Encode timestamp into `ulid[9..0]`:
   `for (i=9; i>=0; i--, t/=32) if (ulid[i] != A[t%32]) { ulid[i]=A[t%32]; same=0; }`
   → `ulid[0]` is the most-significant digit (standard ULID big-endian). Verified.
   `same` stays 1 **only if all 10 timestamp chars are unchanged** (same ms as last call).
5. If `same` (same millisecond as previous call):
   - Increment the random part in place (right-to-left):
     `i=15; while (i>=0 && buf[i]=='Z') buf[i--]='0';`
   - If `i < 0` (all were `Z`): `nanosleep(0, 1234567)` then **recurse** `ulidgen_r(ulid)`.
   - Else find `buf[i]` in the alphabet and set it to the next symbol; return.
   - If `buf[i]` is not a valid alphabet char, fall through to re-randomize.
6. Otherwise (new ms): `getentropy(rnd,16)` (abort on failure);
   `for i in 0..16: buf[i] = A[rnd[i] % 32]`.

### Subtleties that must be preserved
- **Statefulness**: uniqueness/same-ms increment depends on remembering the previous
  ULID. A naive stateless port would lose the "increment within the same millisecond"
  behavior. → Model as a struct holding the last ULID.
- **`same` detection** compares against the *previous* value, not a fresh buffer.
- **Increment** zeroes trailing `Z`s then bumps the first non-`Z`; the chars to the
  right are already `'0'` (correct carry). Guard against the last symbol (`pos+1 < 32`).
- **Entropy**: `getentropy` → Rust `getrandom` crate (portable, no libc-specific call).
- **Recursion on overflow** → translate to a loop or bounded recursion with the same
  1.23 ms sleep.
- **Exit status**: CLI exits non-zero on stdout write error (`exit(!!ferror(stdout))`).

## 3. Dependency mapping (C → Rust)

| Source dependency | Rust counterpart | Notes |
|-------------------|------------------|-------|
| libc `getentropy(2)` | **`getrandom`** crate (or `rand::thread_rng`) | Portable CSPRNG; `getrandom::getrandom(&mut buf)`. |
| libc `getopt` (CLI) | **`clap`** (idiomatic) *or* manual `std::env::args` | `clap` v4 is the idiomatic choice; manual parsing keeps deps minimal. |
| libc `clock_gettime` | `std::time::{SystemTime, UNIX_EPOCH}` | `duration_since(UNIX_EPOCH).as_millis()`. |
| libc `nanosleep` | `std::thread::sleep(Duration)` | 1_234_567 ns. |
| libc `printf/puts/getdelim/setvbuf` | `std::io::{stdout, stdin, BufRead, Write}` | Line-buffering via `BufWriter`/`flush`. |
| (none) | `libc` **not needed** | All functionality covered by std + getrandom (+ optional clap). |

**Recommended dependency set (minimal):** `getrandom = "0.2"` (or `0.3`).
**Optional:** `clap = { version = "4", features = ["derive"] }` for the CLI.

## 4. Target project structure

```
ulidgen/
├── Cargo.toml
├── src/
│   ├── lib.rs        # UlidGen core  (== src/ulid.c + src/ulid.h)
│   └── main.rs       # CLI           (== src/ulidgen.c)
└── tests/
    └── ulid.rs       # integration tests (== tests/test.c)
```

### `src/lib.rs` — core (reference implementation)

```rust
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use getrandom::getrandom;

const B32: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Stateful ULID generator. `last` mirrors the C "reused buffer" state.
#[derive(Default)]
pub struct UlidGen {
    last: [u8; 26],
}

impl UlidGen {
    pub fn new() -> Self { Self { last: [0u8; 26] } }

    pub fn next(&mut self) -> String {
        let mut ulid = self.last;          // start from previous value (C buffer reuse)
        let mut same = true;

        let mut t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        for i in (0..10).rev() {
            let c = B32[(t % 32) as usize];
            if ulid[i] != c { ulid[i] = c; same = false; }
            t /= 32;
        }

        if same {
            // increment the 16-char random region (ulid[10..26]) right-to-left
            let mut i = 25usize;
            while i >= 10 && ulid[i] == b'Z' { ulid[i] = b'0'; i -= 1; }
            if i < 10 {
                // all Z: wait ~1.23 ms and retry (C recursion)
                std::thread::sleep(Duration::from_nanos(1_234_567));
                return self.next();
            }
            if let Some(pos) = B32.iter().position(|&c| c == ulid[i]) {
                if pos + 1 < 32 {
                    ulid[i] = B32[pos + 1];
                    self.last = ulid;
                    return String::from_utf8(ulid.to_vec()).unwrap();
                }
                // else: invalid/edge char -> fall through to re-randomize
            }
        }

        // new millisecond: randomize 16 bytes
        let mut rnd = [0u8; 16];
        if getrandom(&mut rnd).is_err() { std::process::abort(); }
        for i in 0..16 { ulid[10 + i] = B32[(rnd[i] % 32) as usize]; }

        self.last = ulid;
        String::from_utf8(ulid.to_vec()).unwrap()
    }
}
```

### `src/main.rs` — CLI (reference implementation)

```rust
use std::io::{self, BufRead, Write};
use ulidgen::UlidGen;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut n: i64 = 1;
    let mut tflag = false;
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-n" => { n = it.next().and_then(|v| v.parse().ok()).unwrap_or(1); }
            "-t" => { tflag = true; }
            _ => { eprintln!("ulidgen: unknown option {a}"); std::process::exit(2); }
        }
    }

    let mut gen = UlidGen::new();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if tflag {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => { let _ = writeln!(out, "{} {}", gen.next(), l); }
                Err(_) => break,
            }
        }
    } else {
        for _ in 0..n.max(0) { let _ = writeln!(out, "{}", gen.next()); }
    }

    let ok = out.flush().is_ok();
    std::process::exit(if ok { 0 } else { 1 });
}
```

### `tests/ulid.rs` — tests (mirror of `tests/test.c`)

```rust
use ulidgen::UlidGen;

const B32: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn is_valid_ulid(u: &str) -> bool {
    u.len() == 26 && u.bytes().all(|b| B32.as_bytes().contains(&b))
}

#[test]
fn ulid_length() {
    let mut g = UlidGen::new();
    assert_eq!(g.next().len(), 26);
}

#[test]
fn ulid_structure() {
    let mut g = UlidGen::new();
    assert!(is_valid_ulid(&g.next()));
}

#[test]
fn ulid_uniqueness() {
    let mut g = UlidGen::new();
    assert_ne!(g.next(), g.next());
}

#[test]
fn ulid_sortability() {
    let mut g = UlidGen::new();
    let a = g.next();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let b = g.next();
    assert!(a < b);
}
```

### `Cargo.toml`

```toml
[package]
name = "ulidgen"
version = "0.1.0"
edition = "2021"

[dependencies]
getrandom = "0.2"
# clap = { version = "4", features = ["derive"] }   # optional, if preferred over manual parsing

[[bin]]
name = "ulidgen"
path = "src/main.rs"
```

## 5. Translation risks & mitigations

1. **Statefulness / buffer-reuse semantics** — the C function's behavior depends on the
   caller reusing the buffer. *Mitigation:* encapsulate state in `UlidGen.last`; document
   that `next()` is stateful. Tests must use a single instance for the same-ms path.
2. **`same` flag edge cases** — comparing against previous value; a fresh instance
   (`last = [0;26]`) always takes the randomize path (matches C's fresh-buffer behavior).
3. **Increment carry & last-symbol guard** — ensure `pos+1 < 32` so we never write NUL;
   the C code would write `'\0'` if `buf[i]` were the last symbol, but that case is
   unreachable because trailing `Z`s are zeroed first. Keep the guard for safety.
4. **Recursion depth on all-`Z` overflow** — C recurses; keep bounded (sleep + retry).
   In practice the random part is 16 base32 digits, so overflow is astronomically rare.
5. **Entropy portability** — `getentropy` is Linux-specific; `getrandom` crate abstracts
   this and works on all target OSes. `abort()` on failure preserved.
6. **Exit status** — C exits `!!ferror(stdout)`. Rust: check `flush()` result and exit 1
   on error. Preserve `-n` default of 1 and `-t` line-prefixing exactly.
7. **Line buffering** — C uses `setvbuf(_IOLBF)` for `-t`. Rust `BufWriter` + per-line
   `flush` (or rely on `writeln!` to a locked stdout) gives equivalent interactive
   behavior.
8. **`-n` parsing** — C uses `atol` (lenient). Rust `parse::<i64>()` is stricter; guard
   with a sensible fallback to avoid panics on bad input.
9. **Coverage** — source reports ~59–72% coverage; the Rust tests cover the main paths.
   The same-ms increment branch is hard to hit deterministically; consider a unit test
   that pre-seeds `last` (expose a `#[cfg(test)]` setter or make `last` accessible in
   tests) to exercise the increment path explicitly.

## 6. Verification plan
- `cargo build` — compiles lib + bin.
- `cargo test` — runs the 4 mirrored tests (length, structure, uniqueness, sortability).
- Manual: `cargo run -- -n 3` prints 3 ULIDs; `echo hi | cargo run -- -t` prints
  `<ULID> hi`.
- Optional: add a same-ms increment test by seeding `last` to force the branch.
