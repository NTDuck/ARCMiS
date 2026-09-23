# Translation plan: ulidgen (C -> Rust)

## Source overview

- `src/ulid.h` — declares `void ulidgen_r(char[27])`.
- `src/ulid.c` — ULID generation: 10-char base32 timestamp (ms since epoch) +
  16-char random part; same-millisecond in-place increment with recursion.
- `src/ulidgen.c` — CLI: `ulidgen [-n N | -t]`, getopt-based.
- `tests/test.c` — 4 test functions (length, structure, uniqueness,
  sortability); main runs length, uniqueness, sortability (structure test is
  defined but commented out of main).
- `Makefile` — builds `test_1` from `tests/test.c src/ulid.c`, runs it.
- `README` — man page text (public domain, Void Linux).

## Translation strategy

Cargo project in the workspace root:

```
Cargo.toml          package "ulidgen", lib name "ulidgen_r", bin "ulidgen"
src/lib.rs          pub fn ulidgen_r(ulid: &mut [u8; 27])  (fidelity to char[27])
src/main.rs         CLI: -n N (default 1), -t (tag stdin lines)
tests/test.rs       ported tests as #[test] functions
```

- **lib crate `ulidgen_r`**: single public function `ulidgen_r(&mut [u8; 27])`
  mirroring the C signature. The buffer is both input (previous ULID, for the
  same-ms increment logic) and output (26 chars + NUL at index 26).
  - Timestamp: `SystemTime::now().duration_since(UNIX_EPOCH)` -> ms, encoded
    little-endian base32 into indices 9..=0 (most significant digit at 0),
    exactly like the C loop `for (i = 9; i >= 0; i--, t /= 32)`.
  - Same-ms detection: compare the 10 timestamp chars against the existing
    buffer; if all equal, increment the 16-char random part in place
    (Crockford alphabet `0123456789ABCDEFGHJKMNPQRSTVWXYZ`, `Z` wraps to `0`),
    else re-randomize.
  - All-`Z` overflow: `thread::sleep(1234567 ns)` then recurse, as in C.
  - Randomness: 16 bytes via the `getrandom` crate (`getrandom::fill`),
    `abort()` on failure (matches C `getentropy < 0 -> abort()`).
- **binary `ulidgen`**: manual argument parsing (no clap) to keep the
  dependency tree minimal and mimic getopt's lenient behavior:
  `-n` consumes the next arg as a number (default 1), `-t` sets tag mode,
  unknown args are ignored (C switch has no default case).
  - `-t` mode: read stdin lines (keeping the trailing newline), print
    `ULID<space>line`; line-buffered output (Rust's stdout is line-buffered
    on a tty by default; use `flush()` per line to be safe).
  - `-n` mode: print N ULIDs, one per line.
  - Exit 0 on success, non-zero if stdout write fails (mirrors
    `exit(!!ferror(stdout))`).
- **tests**: port each C test function to a `#[test]` in `tests/test.rs`,
  using a fresh zeroed `[0u8; 27]` buffer per call (C tests use uninitialized
  locals, which in practice always take the re-randomize path; zeroed buffer
  is the safe equivalent). Keep the 1.5 ms sleep in the sortability test.
  Include the structure test as a real `#[test]` (it was defined in C but
  not called from main; porting it as a test is a faithful superset and
  cannot weaken anything).

## Core difficulties and candidate approaches

1. **`getentropy` (Linux syscall) -> Rust**
   - A: `getrandom` crate `getrandom::fill(&mut buf)` — direct syscall wrapper,
     closest to C semantics. **Chosen.**
   - B: `rand` crate `thread_rng().fill_bytes` — heavier, adds OS RNG +
     ChaCha machinery; unnecessary.
   - Failure handling: C calls `abort()`; Rust: `std::process::abort()`.

2. **`clock_gettime(CLOCK_REALTIME)` -> Rust**
   - `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` ->
     `as_secs()*1000 + as_nanos()/1_000_000`. No crate needed. **Chosen.**

3. **Same-millisecond increment + recursion**
   - The C function is stateful through the caller's buffer: it reads the
     previous ULID from the same buffer it writes. Rust equivalent:
     `&mut [u8; 27]` parameter; the CLI keeps one buffer across calls,
     exactly like `main` reuses `char ulid[27]`.
   - Recursion on all-`Z` overflow: keep the recursive call (depth is bounded
     by ms ticks, each level sleeps 1.23 ms) — faithful and simple.
     Alternative loop with a `loop { ... }` is possible but recursion matches
     the source 1:1. **Chosen: keep recursion.**
   - Careful with the "invalid char" fall-through: if the random part
     contains a char not in the alphabet, C falls through to re-randomize;
     replicate with `match`/`if let` rather than assuming validity.

4. **`getopt` -> Rust**
   - A: manual `std::env::args` parsing — zero deps, mimics the C switch
     (no default case, `-n` takes `atol`-style value). **Chosen.**
   - B: `clap` — nicer UX but changes behavior (errors on unknown flags) and
     adds a dependency; not needed for this small CLI.

5. **`nanosleep` -> Rust**
   - `std::thread::sleep(Duration::from_nanos(1_234_567))`. **Chosen.**

6. **Test porting**
   - C `assert` -> Rust `assert!`; `strcmp` ordering -> `String` comparison
     (ULIDs are ASCII, so byte order == lexicographic order).
   - `strlen` -> length of the 26-char slice.
   - The C test file's `main` prints progress lines; Rust `#[test]` functions
     are independent — port each as its own `#[test]`, keep the prints as
     `println!` for parity.
   - `cargo test` runs tests in parallel threads by default; the C tests run
     sequentially in one process. The uniqueness/sortability tests each use
     their own buffers, so parallelism does not change their validity.

7. **Buffer/encoding representation**
   - Keep `[u8; 27]` (26 ASCII chars + NUL) for API fidelity; the CLI
     converts to `String` via `std::str::from_utf8` for printing.
     Alternative: `String`-based API — more idiomatic but diverges from the
     C signature. **Chosen: `[u8; 27]`.**

## Verification

- `cargo build` must succeed with no warnings-as-errors issues.
- `cargo test` must pass all ported tests.
- Smoke test: `cargo run -- -n 3` prints 3 valid ULIDs;
  `printf 'a\nb\n' | cargo run -- -t` prints tagged lines.
