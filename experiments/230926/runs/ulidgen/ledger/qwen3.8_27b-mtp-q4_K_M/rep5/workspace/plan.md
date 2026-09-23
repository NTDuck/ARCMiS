# Translation Plan: ulidgen (C -> Rust)

## Goal
Translate the ulidgen ULID generator CLI from C to Rust, buildable and testable
with `cargo test`.

## Core difficulties

1. **`getentropy()` usage** — the C code fills 10 random bytes with
   `getentropy(2)`. Rust std does not expose `getentropy` directly.
   Options:
   - `getrandom` crate (clean, but requires the crate to be available offline).
   - `libc` crate calling `getentropy` (also a dependency).
   - **std-only fallback: read 10 bytes from `/dev/urandom`** via
     `std::fs::File::open("/dev/urandom")` + `read_exact`. Works on Linux
     with zero dependencies.
2. **`clock_gettime(CLOCK_REALTIME)` millisecond timestamp** — replace with
   `SystemTime::now().duration_since(UNIX_EPOCH).as_millis()`, encoded into
   the 6-byte big-endian timestamp prefix (10 Crockford base32 chars).
3. **"Same millisecond" increment logic in `ulidgen_r`** — when the new
   timestamp equals the previous one, the C code increments the base32 random
   part in place (right-to-left carry over Crockford digits), and on overflow
   (all 16 random chars at `Z`) it `nanosleep`s ~1ms and retries. In Rust:
   keep the previous ULID (or its random suffix) in a static/`Mutex`-guarded
   global, decode the 16 base32 chars, add 1 with carry, re-encode; on
   overflow `thread::sleep(Duration::from_millis(1))` and retry.
4. **`getopt` CLI (`-n N`, `-t`)** — either `clap` (dependency) or manual
   `std::env::args()` parsing. Manual parsing is trivial for two flags and
   keeps the build dependency-free.
5. **`getdelim` line reading** (used by the `-t`/stdin mode) — replace with
   `std::io::BufRead::lines()` over `stdin()`.
6. **Tests** — translate `tests/test.c` assertions into Rust `#[test]`
   functions in `tests/test.rs`:
   - ULID string length is 26;
   - every character is a valid Crockford base32 char (0-9, A-Z minus
     I/L/O/U);
   - consecutive ULIDs are unique;
   - ULIDs generated after a 1.5 ms delay sort lexicographically
     (timestamp ordering).

## Candidate approaches

| Approach | Pros | Cons |
|---|---|---|
| **std-only** (recommended) | Builds offline, no network, no lockfile | `/dev/urandom` is Unix-specific (fine for this project) |
| `getrandom` crate | Portable, idiomatic | Needs registry access / vendored crate |
| `libc` crate | Direct `getentropy` | Needs registry access; FFI boilerplate |

**Recommendation: std-only.** Network may be unavailable, so prefer a
zero-dependency build. Randomness via `/dev/urandom` (Linux target, matching
the C code's `getentropy`), time via `SystemTime::now`, CLI via manual arg
parsing, line reading via `BufRead::lines`. If the build environment turns
out to have a crate cache, `getrandom` could be swapped in later without
changing the public API.

## Layout

```
Cargo.toml        # package "ulidgen", no dependencies
src/lib.rs        # pub fn ulidgen_r(ulid: &mut [u8; 27])  (26 chars + NUL,
                  # mirroring the C signature) and/or pub fn ulid() -> String;
                  # Crockford base32 tables, timestamp + random encoding,
                  # same-millisecond increment with overflow retry
src/main.rs       # CLI: -n N (count), -t (read lines from stdin, one ULID
                  # per input line); prints ULIDs to stdout
tests/test.rs     # integration tests mirroring tests/test.c
```

## Notes
- Keep the Crockford base32 alphabet exactly: `0123456789ABCDEFGHJKMNPQRSTVWXYZ`.
- The increment-with-carry must operate on the 16 random chars only,
  preserving the 10-char timestamp prefix.
- Global "last ULID" state: `static LAST: Mutex<Option<[u8; 27]>>` (or
  `std::sync::OnceLock<Mutex<...>>`) to make the same-millisecond logic
  thread-safe.
