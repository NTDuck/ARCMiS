# Translation plan: ulidgen (C → Rust)

## Source overview

- `src/ulid.h` — declares `void ulidgen_r(char[27]);`
- `src/ulid.c` — `ulidgen_r`: fills a caller-owned 27-byte buffer (26 chars + NUL)
  with a ULID. State is implicit in the buffer itself (the previous ULID), which
  drives the `same` flag.
- `src/ulidgen.c` — CLI: `getopt("n:t")`, `-n N` prints N ULIDs, `-t` prefixes
  each stdin line with a ULID. Line-buffered stdout, exits non-zero on ferror.
- `tests/test.c` — standalone test main: length, structure (commented out),
  uniqueness, sortability (1.5 ms sleep between two generations).
- `Makefile`, `README` (man page), `coverage_report.json` — build/docs, no logic.

## Translation strategy

1. One Cargo package, **zero external dependencies** (std only), so
   `cargo test` works offline.
2. `src/lib.rs` — faithful port of `ulidgen_r` as
   `pub fn ulidgen_r(ulid: &mut [u8; 27])`. Keeping the caller-owned buffer
   mirrors the C state model exactly: the CLI reuses one buffer (shared state),
   the tests use separate buffers (independent state). This is the key design
   decision — do NOT introduce a hidden global/static in Rust.
3. `src/main.rs` — port of `ulidgen.c`: manual arg parsing, `-n N` loop,
   `-t` stdin tagging, non-zero exit on stdout write error.
4. `tests/test.rs` — port of `tests/test.c` as `#[test]` functions (cargo runs
   them in parallel threads; each uses its own buffer, so no interference).
5. Preserve C behavior bit-for-bit: same base32 alphabet, same big-endian
   base32 timestamp encoding (loop `i = 9..=0`, `t /= 32`), same increment
   algorithm, same 1 234 567 ns sleep, same abort-on-entropy-failure.

## Core difficulties and chosen approaches

### 1. `getentropy(rnd, 16)` without external crates
C aborts on failure.
- **Chosen:** open `/dev/urandom` with `std::fs::OpenOptions` (unix
  `OpenOptionsExt`), `read_exact` 16 bytes, `std::process::abort()` on error.
  std-only, no `libc`/`getrandom` crate needed.
- Alternatives: `extern "C" getentropy` via the `libc` crate (adds a
  dependency, rejected); reading `/dev/random` (blocking, wrong semantics).

### 2. `clock_gettime(CLOCK_REALTIME)` → milliseconds
C: `t = tv.tv_sec*1000 + tv.tv_nsec/1000000` (truncated ms since epoch).
- **Chosen:** `SystemTime::now().duration_since(UNIX_EPOCH).as_millis() as u64`
  — identical truncation semantics.
- Alternative: `libc::clock_gettime` (dependency, rejected).

### 3. `nanosleep({0, 1234567})` (all-Z overflow restart)
- **Chosen:** `std::thread::sleep(Duration::from_nanos(1_234_567))` then
  recurse into `ulidgen_r` exactly like the C recursion.
- Alternative: spin-wait until the ms counter changes (changes observable
  behavior, rejected).

### 4. In-place increment of the random part (chars 10..25)
C: from index 15 down, replace trailing `'Z'` with `'0'`; if all 16 are `'Z'`
sleep+recurse; else advance the rightmost non-`'Z'` char to its successor in
the alphabet; if the char is not in the alphabet at all, fall through to
re-randomize.
- **Chosen:** operate on `&mut ulid[10..26]` with a `usize` index guarded
  against underflow (C's `i < 0` check). Look up the char in the alphabet
  string; if found at position `p < 31`, write `alphabet[p+1]`; otherwise fall
  through to the entropy path. Preserve the fall-through-on-invalid-char
  behavior (matters for garbage-initialized buffers).
- Pitfall: Rust `usize` underflow — use `checked_sub`/bounds checks instead of
  `i--` past 0.

### 5. `same` flag semantics (state in the caller's buffer)
`same` starts 1; the timestamp loop sets it to 0 as soon as any of chars 0..9
differs from the previous call's value. `same == 1` ⇒ increment in place;
otherwise ⇒ fresh 16 random chars.
- **Chosen:** replicate exactly: compare each computed timestamp char against
  the existing buffer byte, set `same = false` on first mismatch. Because the
  buffer is the state, the CLI (one reused buffer) and the tests (separate
  buffers) behave exactly as in C.
- Rejected: a `struct UlidGen { last_ms, last_random }` state object — cleaner
  Rust, but it breaks the C test's "two separate buffers ⇒ two independent
  streams" behavior and the CLI's single-buffer behavior would need extra
  plumbing.

### 6. `getopt(argc, argv, "n:t")` → manual parsing
- **Chosen:** hand-rolled loop over `std::env::args()`:
  `-n` takes the next arg (also accept the attached `-nN` form, as getopt
  does); `-t` sets the flag; unknown options print an error to stderr and are
  skipped (C's switch has no `default`, so getopt's error is printed and
  parsing continues). `n` parsed like `atol`: leading digits, 0 on failure.
- Rejected: `clap`/`structopt` (external dependency, different error
  semantics).

### 7. Commented-out test in `tests/test.c`
`test_ulid_structure` is defined but commented out of `main()`.
- **Chosen:** port it as an *active* `#[test]` in `tests/test.rs`. It is a
  valid assertion (26 chars, all in the alphabet) and always passes; keeping
  it active only adds coverage and does not weaken anything. (Alternative:
  mirror the C file by leaving it `#[ignore]`d — noted, not chosen.)
- The other three tests port 1:1: length, uniqueness (two separate buffers),
  sortability (1.5 ms `thread::sleep` between generations, assert
  `ulid1 < ulid2` — valid because the timestamp is encoded big-endian, so
  lexicographic order = time order).

### 8. I/O details of the CLI
- `setvbuf(stdout, _IOLBF, 0)` → Rust `Stdout` is line-buffered already;
  flush after each line to be safe.
- `getdelim` keeps the trailing newline → use `BufRead::read_line` (NOT
  `lines()`, which strips it) so `printf("%s %s", ulid, line)` maps to
  `write!(stdout, "{} {}", ulid, line)` with the newline intact, including a
  final line without newline.
- `exit(!!ferror(stdout))` → propagate write errors and
  `std::process::exit(1)` on failure, `exit(0)` otherwise.

## Verification

- `cargo build` and `cargo test` must pass.
- Parity spot-checks against the C binary: `ulidgen -n 3` outputs 26-char
  alphabet-valid, lexicographically increasing lines; `-t` prefixes stdin
  lines; consecutive same-millisecond ULIDs differ only in the random part
  (incremented, not re-randomized).
