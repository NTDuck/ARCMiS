# Notes

## Findings

(empty for now)

## Core difficulties identified (planning)

1. `getentropy()` -> no direct std equivalent; std-only fallback is reading
   `/dev/urandom` (or use `getrandom`/`libc` crates if available offline).
2. `clock_gettime(CLOCK_REALTIME)` ms timestamp -> `SystemTime::now()`
   `duration_since(UNIX_EPOCH).as_millis()`.
3. Same-millisecond increment of the base32 random part in `ulidgen_r`,
   with `nanosleep` + retry on overflow -> in-place carry over 16 Crockford
   digits, `thread::sleep(1ms)` + retry; needs thread-safe global state.
4. `getopt` CLI (`-n N`, `-t`) -> manual `std::env::args()` parsing
   (or `clap`).
5. `getdelim` line reading -> `BufRead::lines()`.
6. Tests in `tests/test.c` -> Rust `#[test]` functions in `tests/test.rs`
   (length 26, valid Crockford chars, uniqueness, lexicographic sortability
   after 1.5 ms delay).

## Decision

- Recommended approach: **std-only, zero dependencies** (offline-safe).
- Layout: `Cargo.toml`, `src/lib.rs`, `src/main.rs`, `tests/test.rs`.
- Note: workspace was empty at planning time; source files (Makefile,
  README, src/ulid.c, src/ulid.h, src/ulidgen.c, tests/test.c) were not
  present, so the plan is based on the described project structure.

## Implementation results (ledger worker)

### API choice
- `pub fn ulidgen_r(ulid: &mut [u8; 27])` in `src/lib.rs` — mirrors the C
  signature exactly: 26 ULID chars + NUL at index 26 (always reset to 0).
  The buffer is the state: the previous ULID is read from it (reusable-buffer
  semantics), so the function is stateless and thread-safe (no statics).
- Randomness: 16 bytes from `/dev/urandom` (`File::open` + `read_exact`),
  `panic!` on failure (mirrors C `getentropy` + `abort()`).
- Time: `SystemTime::now().duration_since(UNIX_EPOCH).as_millis()`.
- Overflow retry: `thread::sleep(1_234_567 ns)` + loop (mirrors C
  `nanosleep` + recursion).

### Bugs found and fixed during this pass
1. **Timestamp never written to the output buffer.** `ulidgen_r` computed the
   10-char timestamp into a local array but only wrote the 16 random chars
   into `ulid[10..26]`, so every ULID started with "0000000000" and
   `test_ulid_sortability` failed. Fixed by copying the timestamp into
   `ulid[0..10]` before the same-millisecond check (as the C code does).
2. **Doctest compile failure.** The signature snippet in the module doc
   comment was compiled as a doctest; marked it `rust,ignore`.

### Deviations from C semantics
- `getentropy` -> `/dev/urandom` (std-only, per plan); `abort()` -> `panic!`.
- `nanosleep` + recursion -> `thread::sleep` + `loop` (identical behavior).
- The C `same` flag is derived from the timestamp chars only; the Rust code
  additionally requires the previous buffer to be a fully valid Crockford
  string before taking the increment path. If the previous random part
  contains invalid chars, C falls through to randomize after a failed
  `strchr`; Rust randomizes directly. Observable result is identical.
- CLI: manual `std::env::args` parsing (no getopt); `-t` mode flushes stdout
  after each line (line-buffered, as required).

### Build / test results
- `cargo build`: success (zero dependencies, edition 2021).
- `cargo test`: 4/4 integration tests pass (test_ulid_length,
  test_ulid_structure, test_ulid_uniqueness, test_ulid_sortability),
  1/1 doctest passes (1 signature snippet ignored).
- CLI smoke test: `-n 5` prints 5 ULIDs with same-ms suffix increments
  (…PXD, PXE, PXF, PXG, PXH); `-t` prefixes stdin lines with ULID + space;
  bad args exit 1 with usage; success exits 0.

## Verification round (no code changes)
- `cargo build`: Finished `dev` profile [unoptimized + debuginfo] target(s); exit 0; no warnings (re-verified with a forced recompile after touching sources).
- `cargo test`: all ok — lib 0 tests, main 0 tests, tests/test.rs 4 passed (test_ulid_uniqueness, test_ulid_length, test_ulid_structure, test_ulid_sortability), doc-tests 1 passed / 1 ignored.
- `cargo run -q -- -n 3`: 3 ULIDs, each 26 chars, Crockford base32 alphabet, first 10 chars decode to 2026-09-23 07:56:23 UTC (matches system clock).
- `echo "hello" | cargo run -q -- -t`: prints "<ulid> hello".
- `cargo run -q -- -x`: exit=1 with usage message.
- Files present: Cargo.toml, src/lib.rs, src/main.rs, tests/test.rs.
