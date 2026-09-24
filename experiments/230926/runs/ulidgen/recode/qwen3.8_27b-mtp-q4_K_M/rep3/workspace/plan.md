# ulidgen — C → Rust Implementation Plan

Source: public-domain C `ulidgen` (Leah Neukirchen). Target: Rust cargo package
`ulidgen` (see `Cargo.toml`, already present). Test command: `cargo test`.

The skeleton files (`src/lib.rs`, `src/main.rs`, `tests/ulid.rs`) already exist
and compile (`cargo build` / `cargo test --no-run` pass). `ulidgen_r` in
`src/lib.rs` is a `todo!()` stub with detailed TODO notes; the rest of the
skeleton is written and may be refined while implementing.

## Name mapping (C → Rust)

| C symbol | Rust symbol | Notes |
|---|---|---|
| `ulidgen_r(char[27])` | `ulidgen_r(buf: &mut [u8; 27])` | Name preserved; C array decays to pointer, Rust uses `&mut [u8; 27]` to keep the same-buffer contract explicit. |
| `b32alphabet` (static local) | `B32_ALPHABET` (pub const) | Promoted to a public constant so tests can validate structure. |
| `ulidgen` (binary / `main`) | `main()` in `src/main.rs` | Binary name stays `ulidgen` (cargo package name). |
| `getopt` loop | `Args` struct (clap derive) + `parse_long` | New names; `-n`/`-t` flags preserved. |
| `is_valid_ulid` (test helper) | `is_valid_ulid(ulid: &str) -> bool` | Name preserved. |
| `test_ulid_length` | `ulid_length` | `test_` prefix dropped (Rust `#[test]` convention). |
| `test_ulid_structure` | `ulid_structure` | Same. |
| `test_ulid_uniqueness` | `ulid_uniqueness` | Same. |
| `test_ulid_sortability` | `ulid_sortability` | Same. |
| `main` (test runner) | `#[test]` attributes | No explicit test runner needed. |
| `getentropy` | `getrandom::fill` | Crate API. |
| `clock_gettime` | `SystemTime::now().duration_since(UNIX_EPOCH)` | std. |
| `nanosleep` | `std::thread::sleep` | std. |
| `getdelim` | `stdin().lock().lines()` | std. |
| `setvbuf(_IOLBF)` | `LineWriter` | std. |
| `abort()` | `std::process::abort()` | Preserved. |
| `exit(!!ferror(stdout))` | `std::process::exit(1)` on IO error | Preserved exit-status contract (0 success / 1 error). |

## Part A — source files (bottom-up dependency order)

### A1. `src/lib.rs` (depends on: `getrandom`, std only)
Implement `ulidgen_r(buf: &mut [u8; 27])` exactly per the TODO notes in the
skeleton, preserving C semantics:
1. `buf[26] = 0` (NUL terminator).
2. Timestamp ms: `t = as_secs()*1000 + as_nanos()/1_000_000` (u64).
3. Encode into `buf[0..10]` with `for i in (0..10).rev() { buf[i] = B32[t % 32]; t /= 32; }`,
   tracking `same` (set false whenever the new char differs from the previous
   buffer content at that position — compare BEFORE overwriting).
4. If `same`: scan `i` from 15 down while `buf[i] == b'Z'` → `b'0'`;
   - if `i < 0`: `thread::sleep(Duration::from_nanos(1_234_567))` and recurse
     `ulidgen_r(buf)`;
   - else advance `buf[i]` to its successor in `B32_ALPHABET`;
   - if `buf[i]` is not in the alphabet, fall through to re-randomize.
5. Random path: `getrandom::fill(&mut rnd)` (16 bytes), on error
   `std::process::abort()`; then `buf[i] = B32[rnd[i] % 32]` for `i in 0..16`
   (preserve modulo bias as-is).
`ulid() -> String` is already implemented in the skeleton (fresh zeroed buffer);
keep it and its doc note about the increment-path caveat.

### A2. `src/main.rs` (depends on: A1, `clap`, std)
Skeleton already implements the CLI; verify/refine:
- clap `Args`: `-n N` (i64, default 1, `parse_long` mirroring `atol`), `-t`.
- One shared `[u8; 27]` buffer across all `ulidgen_r` calls (C semantics).
- `-t` mode: `LineWriter` over stdout (line-buffered like `_IOLBF`),
  `stdin().lock().lines()`, print `"{ulid} {line}"` + newline (C printed
  `"%s %s"` with the line's trailing `\n`; `lines()` strips it, so restore it).
- Default mode: print `n` ULIDs, one per line.
- Exit status: any write/read error → `std::process::exit(1)`
  (mirrors `fflush(0); exit(!!ferror(stdout));`); success → 0.

## Part B — test files (bottom-up dependency order)

### B1. `tests/ulid.rs` (depends on: A1)
Skeleton already mirrors `tests/test.c`; verify/refine:
- `is_valid_ulid(ulid: &str) -> bool` — length 26 + all chars in `B32_ALPHABET`.
- `gen(buf: &mut [u8; 27]) -> String` helper — drives `ulidgen_r` on a shared
  buffer so the same-millisecond increment path works (C tests reuse one
  `char ulid[27]`).
- `ulid_length`: generated ULID is 26 chars.
- `ulid_structure`: all chars in the Crockford base32 alphabet.
- `ulid_uniqueness`: two consecutive ULIDs (same buffer) differ.
- `ulid_sortability`: after a 2 ms sleep (C used 1.5 ms; 2 ms for CI safety),
  the second ULID sorts after the first.

## Verification

1. `cargo build` — no errors.
2. `cargo test` — all 4 tests pass.
3. Smoke: `cargo run -- -n 3` prints 3 ULIDs; `printf 'a\nb\n' | cargo run -- -t`
   prefixes each line; both exit 0.
