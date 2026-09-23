# Translation Plan: ulidgen (C → Rust)

## Source overview (assets/ReCodeAgent/data/tool_projects/crust/ulidgen/c)

| C file | Role |
|---|---|
| `src/ulid.h` | Declares `void ulidgen_r(char[27])` |
| `src/ulid.c` | Core ULID generation: ms timestamp → Crockford base32 (10 chars), random 16 bytes → base32 (16 chars); same-millisecond in-place increment with `Z`→`0` wraparound, invalid-char re-randomization, and 1.23ms `nanosleep` + recursion on full carry |
| `src/ulidgen.c` | CLI: `getopt` for `-n N` / `-t`; `-t` = line-tagging mode (getdelim loop, line-buffered stdout, exit `!!ferror(stdout)`); else print N ULIDs |
| `tests/test.c` | 4 tests: length, structure (commented out in main), uniqueness, sortability (1.5ms sleep) |
| `Makefile` | Builds `test_1` from `tests/test.c src/ulid.c` and runs it |
| `README` | Man page text (public domain, Leah Neukirchen) |
| `coverage_report.json` | Per-file line/coverage stats (informational) |

## Target layout (workspace root)

```
Cargo.toml          # package `ulidgen`, edition 2021, no external deps (std-only)
src/lib.rs          # `pub fn ulidgen_r(ulid: &mut [u8; 27])` — port of src/ulid.c
src/main.rs         # port of src/ulidgen.c (CLI + line-tagging mode)
tests/test.rs       # port of tests/test.c (4 tests, structure test kept active)
```

## Strategy

1. **std-only, hermetic build.** No `rand`, no `clap`, no `libc`. Everything needed is in
   `std`: `SystemTime`/`Instant` for time, `std::fs::File::open("/dev/urandom")` for entropy,
   `std::thread::sleep` for the fallback delay, manual `argv` parsing for `-n`/`-t`.
   This keeps `cargo test` offline and reproducible.

2. **Faithful port of `ulidgen_r`.** Keep the exact Crockford alphabet
   `"0123456789ABCDEFGHJKMNPQRSTVWXYZ"`, the 10-char timestamp encoding
   (`t /= 32` loop, `same` flag), the in-place increment (scan from index 15 down while
   `== 'Z'` → set `'0'`; if carry falls off, sleep ~1.23ms and recurse; else advance the
   char to its successor in the alphabet; if the char is not in the alphabet, fall through
   to full re-randomization), and the 16-byte → 16-char random encoding (`b % 32`).
   Represent the buffer as `[u8; 27]` (26 chars + NUL) to mirror the C API, or a
   `String`/`[u8; 26]` — decision: use `[u8; 27]` with NUL to stay closest to the C
   signature and make the test port mechanical.

3. **CLI port.** Parse `argv` manually: `-n N` (parse with `str::parse::<i64>()`,
   matching `atol` leniency — accept leading junk gracefully), `-t` flag.
   Line-tagging mode: read stdin line by line (`BufRead::lines` or manual `read_line`
   to preserve the trailing newline exactly as `getdelim` does), print `"{ulid} {line}"`
   without adding a newline. Exit code: 0 on success, 1 if stdout write failed
   (track write errors; Rust panics on broken pipe by default — catch via
   `std::process::exit` on error rather than panicking, to mirror `exit(!!ferror(stdout))`).

4. **Tests.** Port all four tests into `tests/test.rs` (integration test calling
   `ulidgen::ulidgen_r`). Keep the structure test active (it was only commented out in
   the C `main`, the function itself exists). Sortability test: generate, sleep 1.5ms,
   generate, assert `ulid1 < ulid2` lexicographically.

## Core difficulties & candidate approaches

1. **Millisecond timestamp (C `clock_gettime(CLOCK_REALTIME)`)**
   - Rust std: `SystemTime::now().duration_since(UNIX_EPOCH)` → `as_secs()*1000 +
     subsec_millis()`. Same wall-clock source, same ms precision. No crate needed.
   - Alternative: `libc::clock_gettime` — rejected (external dep, no benefit).

2. **Random bytes (C `getentropy`)**
   - std-only: `std::fs::File::open("/dev/urandom")` + `read_exact(&mut [u8; 16])`.
     On Linux this is equivalent in quality to `getentropy` (both kernel CSPRNG).
     On error: `std::process::abort()` to mirror C `abort()`.
   - Alternative: `rand` crate — rejected to keep the build hermetic/offline.
   - (The task's `std::collections::hash_map` hint is a red herring — it is not a
     source of randomness.)

3. **`nanosleep` fallback (1234567 ns ≈ 1.23ms)**
   - Rust std: `std::thread::sleep(Duration::from_nanos(1_234_567))`, then recurse
     exactly as the C code does. `thread::sleep` granularity is fine (it sleeps at
     least the requested time, same guarantee as `nanosleep`).

4. **In-place base32 increment (the trickiest logic)**
   - Port line-by-line:
     - `same` flag: set false the first time a timestamp char differs from the
       previous value (C compares against the *existing* buffer contents, i.e. the
       previously generated ULID — the buffer is reused across calls).
     - Increment: `i = 15; while i >= 0 && buf[i] == b'Z' { buf[i] = b'0'; i -= 1; }`
       (mind Rust's unsigned index — use `i64` or a checked loop).
     - If `i < 0`: sleep 1.23ms, recurse `ulidgen_r(buf)`, return.
     - Else look up `buf[i]` in the alphabet; if found, `buf[i] = alphabet[pos+1]`,
       return; if not found (invalid char), fall through to re-randomization.
   - Fidelity note: the C code's `strchr` lookup means only valid-alphabet chars
     increment; any other byte triggers full re-randomization. Preserve that exactly.

5. **CLI parsing (`getopt "n:t"`)**
   - std-only manual loop over `std::env::args()`: handle `-n N`, `-t`, stop at `--`.
     `atol` semantics: parse leading digits, default 0 on garbage — emulate with
     `trim_start_matches('-').parse().unwrap_or(0)`-style leniency.
   - Alternative: `clap` — rejected (hermeticity; the flag set is trivial).

6. **Line-tagging mode**
   - `getdelim` keeps the trailing newline in the buffer; C prints `"%s %s"` so the
     newline is preserved. Rust: `stdin.lock().read_line(&mut buf)` in a loop (returns
     0 on EOF, preserves `\n`), print `ulid + " " + line`. Avoid `BufRead::lines()`
     which strips the newline (would change output for lines without trailing `\n`).
   - Line-buffered stdout: `setvbuf(_IOLBF)` — in Rust, flush after each line
     (`writeln!`/`write!` + `flush`) to get equivalent interactive behavior.
   - Exit code: C does `exit(!!ferror(stdout))`. Rust: collect write errors
     (e.g. broken pipe) and `std::process::exit(1)` on error; also install a broken-pipe
     handler or catch the panic to avoid a panic backtrace.

7. **Tests port**
   - `tests/test.rs` integration test: `is_valid_ulid` helper (length 26 + alphabet
     check), then `test_ulid_length`, `test_ulid_structure`, `test_ulid_uniqueness`,
     `test_ulid_sortability` (with `thread::sleep(1.5ms)`). All map 1:1 to Rust
     `#[test]` fns; `assert!` replaces `assert()`.

8. **External crates?**
   - Recommendation: **std-only**. The whole program needs time, /dev/urandom,
     sleep, and two flags — all in std. Hermetic `cargo test` with zero network
     dependency is the safest choice for this environment.

## Risks / gotchas

- Rust `thread::sleep` may sleep longer than requested — fine, C `nanosleep` has the
  same property.
- Broken-pipe panic in Rust vs silent `ferror` in C — must convert to exit(1).
- Unsigned underflow in the decrement loop — use `i64` index.
- The `same` flag depends on buffer reuse across calls (static state in C is the
  caller's buffer) — keep the same `&mut [u8; 27]` API so tests/CLI reuse one buffer.
