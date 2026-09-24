# ulidgen C → Rust translation plan

Source: public-domain C CLI `ulidgen` (generate ULIDs / tag stdin lines).
Target: Cargo package `ulidgen` (edition 2021), deps: `clap` (derive), `getrandom`.
Test command: `cargo test`.

## Phase 1 — Fragment inventory (source)

| Source file | Symbol | Kind | Notes |
|---|---|---|---|
| src/ulid.h | `ulidgen_r` | fn decl | `void ulidgen_r(char[27])` |
| src/ulid.c | `ulidgen_r` | fn | core: ms-timestamp encode (10 chars), same-ms in-place increment of 16-char random part, `nanosleep`+recurse on overflow, `getentropy` fill, `abort()` on entropy failure |
| src/ulid.c | `b32alphabet` | static const | Crockford base32, 32 chars |
| src/ulidgen.c | `main` | fn | getopt `-n N` (default 1) / `-t`; persistent `char ulid[27]`; `setvbuf(_IOLBF)`; `getdelim` loop; `exit(!!ferror(stdout))` |
| tests/test.c | `is_valid_ulid` | fn | length 26 + alphabet check |
| tests/test.c | `test_ulid_length` | fn | |
| tests/test.c | `test_ulid_structure` | fn | (commented out in C main; enable in Rust) |
| tests/test.c | `test_ulid_uniqueness` | fn | |
| tests/test.c | `test_ulid_sortability` | fn | 1.5 ms `nanosleep` between calls |

## Phase 2 — Name mapping (C → Rust)

| C symbol | Rust symbol | Reason for change |
|---|---|---|
| `ulidgen_r` | `ulidgen_r` | preserved (lib fn, `&mut [u8; 27]`) |
| `b32alphabet` | `B32_ALPHABET` | Rust const naming convention (SCREAMING_SNAKE) |
| `main` (src/ulidgen.c) | `main` | preserved |
| getopt `n` / `tflag` | `Cli::count` / `Cli::tag` | clap derive struct fields (idiomatic replacement of getopt) |
| `is_valid_ulid` | `is_valid_ulid` | preserved |
| `test_ulid_*` | `test_ulid_*` | preserved |
| `getentropy` | `getrandom::getrandom` | crate API name |
| `clock_gettime(CLOCK_REALTIME)` | `SystemTime::now()` | std API |
| `nanosleep` | `std::thread::sleep` | std API |
| `abort()` | `std::process::abort()` | std API, same semantics |
| `exit(!!ferror(stdout))` | `std::process::exit(1)` on write error | Rust idiom |

No other names change; all public/test symbol names are preserved.

## Phase 3 — Skeleton (done)

Compilable stubs already in the workspace (verified with `cargo check --all-targets`):
- `src/lib.rs` — `B32_ALPHABET`, `ulidgen_r`, `ulid`, unit tests (stubs with `todo!()`)
- `src/main.rs` — clap `Cli`, `main` stub
- `tests/ulid.rs` — `is_valid_ulid` + 4 integration test stubs

## Phase 4 — Implementation plan

### Part A — source files (bottom-up dependency order)

1. **`src/lib.rs`** (no in-project deps; deps: `getrandom`, `std::time`, `std::thread`)
   - `ulidgen_r(buf: &mut [u8; 27])`:
     - `buf[26] = 0` (NUL, API fidelity).
     - ms timestamp: `SystemTime::now().duration_since(UNIX_EPOCH)` → `secs*1000 + nanos/1_000_000` as `u64`.
     - Encode 10 chars: loop `i = 9..=0`, `t /= 32`, `buf[i] = B32_ALPHABET[(t % 32) as usize]`; track `same` (all 10 unchanged from previous contents).
     - If `same`: increment random part `buf[10..26]` in place — scan from index 25 down while byte == b'Z' → set b'0'; if all wrapped: `std::thread::sleep(Duration::from_nanos(1_234_567))` and recurse `ulidgen_r(buf)`; else if current byte is in the alphabet, set it to the next alphabet char and return; if not in alphabet, fall through to re-randomize.
     - Else: `let mut rnd = [0u8; 16]; getrandom::getrandom(&mut rnd).unwrap_or_else(|_| std::process::abort());` then `buf[10+i] = B32_ALPHABET[(rnd[i] % 32) as usize]`.
   - `ulid() -> String`: fresh `[0u8; 27]`, call `ulidgen_r`, `String::from_utf8_lossy(&buf[..26]).into_owned()`.
   - Fill the 4 unit tests (length, structure, uniqueness, sortability with 1.5 ms sleep).
   - Key risk: keep the increment path faithful — it depends on the caller passing the same persistent buffer.

2. **`src/main.rs`** (depends on `src/lib.rs`)
   - One persistent `let mut ulid = [0u8; 27];` across all calls (mirrors C `char ulid[27]`).
   - `-t` mode: `stdin().lock().lines()`; `lines()` strips the trailing newline, so print `"{ulid_str} {line}\n"` to match `getdelim` byte-for-byte; flush after each line (equivalent of `setvbuf(_IOLBF)`).
   - Default mode: loop `count` times, print each ULID + `\n`.
   - Any stdout write/flush error → `eprintln!` + `std::process::exit(1)` (equivalent of `exit(!!ferror(stdout))`).

### Part B — test files (bottom-up dependency order)

1. **`tests/ulid.rs`** (depends on `src/lib.rs`)
   - `is_valid_ulid(s: &str) -> bool`: `s.len() == 26` and every byte in `B32_ALPHABET`.
   - `test_ulid_length`: `ulid().len() == 26`.
   - `test_ulid_structure`: `is_valid_ulid(&ulid())`.
   - `test_ulid_uniqueness`: two consecutive `ulid()` calls differ.
   - `test_ulid_sortability`: `ulid()`, sleep 1.5 ms, second `ulid()` sorts after first.

### Verification

- `cargo check --all-targets` (skeleton already passes)
- `cargo test` (all unit + integration tests green)
- Smoke: `cargo run -- -n 3` (3 lines, 26 chars, consecutive), `printf 'a\nb\n' | cargo run -- -t` (ULID-prefixed lines, trailing newline preserved).
