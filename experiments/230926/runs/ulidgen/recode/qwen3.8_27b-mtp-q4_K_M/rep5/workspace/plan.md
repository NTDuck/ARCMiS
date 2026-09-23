# ulidgen — C → Rust Implementation Plan

Test command: `cargo test` (plus `cargo check --all-targets` after each step).

## 1. Fragment extraction (source inventory)

| Source file | Fragment | Kind |
|---|---|---|
| `src/ulid.h` | `void ulidgen_r(char[27]);` | declaration (no separate target fragment; folded into lib) |
| `src/ulid.c` | `ulidgen_r` | function — core ULID generator |
| `src/ulidgen.c` | `main` | function — CLI (`-n N` / `-t`) |
| `tests/test.c` | `is_valid_ulid` | helper function |
| `tests/test.c` | `test_ulid_length` | test |
| `tests/test.c` | `test_ulid_structure` | test (defined, disabled in C `main`; enable in Rust) |
| `tests/test.c` | `test_ulid_uniqueness` | test |
| `tests/test.c` | `test_ulid_sortability` | test |
| `tests/test.c` | `main` | test runner (replaced by `cargo test`) |

Non-code files: `Makefile` (replaced by Cargo), `README` (man page; keep as-is),
`coverage_report.json` (metadata only, not ported).

## 2. Name mapping (C → Rust)

| C symbol | Rust symbol | Reason for change |
|---|---|---|
| `ulidgen_r` | `ulidgen` (in crate `ulidgen`) | `_r` reentrant suffix dropped; crate name already carries the name. Signature changes: `void ulidgen_r(char[27])` → `pub fn ulidgen(prev: Option<&str>) -> String` — the C buffer-reuse trick for same-ms detection is replaced by an explicit `prev` parameter (design decision). |
| — (no C counterpart) | `ulidgen_fresh` | convenience wrapper `ulidgen(None)`; new, not a rename. |
| `b32alphabet` (local) | `B32_ALPHABET` (pub const) | Rust constant naming convention; promoted to a public const so tests can use it. |
| `main` (src/ulidgen.c) | `main` (src/main.rs) | preserved. |
| `is_valid_ulid` | `is_valid_ulid` | preserved (duplicated in lib unit tests and integration tests, as in C). |
| `test_ulid_length` / `test_ulid_structure` / `test_ulid_uniqueness` / `test_ulid_sortability` | same names | preserved. |
| `getentropy` | `rand::random::<[u8;16]>()` | std has no getentropy; `rand` 0.9 is the chosen CSPRNG counterpart. |
| `clock_gettime(CLOCK_REALTIME)` | `SystemTime::now().duration_since(UNIX_EPOCH)` | std. |
| `nanosleep` | `std::thread::sleep(Duration::from_nanos(1_234_567))` | std. |
| `getopt("n:t")` | manual `std::env::args()` loop | keep dependency-light (design decision). |
| `getdelim` + `setvbuf(_IOLBF)` | `BufRead::lines()` + per-line `flush()` | std; explicit flush preserves line-buffering semantics. |
| `abort()` (entropy failure) | `panic!` / `std::process::abort()` | preserve fail-hard. |
| `exit(!!ferror(stdout))` | `process::exit(1)` on write/flush error | preserve exit-status contract. |

## 3. Skeleton status

Skeleton files already exist and compile (`cargo check --all-targets` passes,
warnings only): `src/lib.rs`, `src/main.rs`, `tests/ulid.rs`. Each stub carries a
`TODO(port)` comment describing exactly what to implement. Implementers fill the
stubs in place; do not rename the public API.

## 4. Part A — source files, bottom-up dependency order

### A1. `src/lib.rs` (depends on: nothing; ports `src/ulid.h` + `src/ulid.c`)

Implement `pub fn ulidgen(prev: Option<&str>) -> String`:

1. `let mut ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;`
2. Encode 10 timestamp chars, most-significant first:
   `for i in (0..10).rev() { push(B32_ALPHABET[(ms % 32) as usize]); ms /= 32; }`
   (C loop `for (i = 9; i >= 0; i--, t /= 32)`).
3. Same-ms detection: `prev` is `Some` and `prev[..10] == timestamp part`.
4. If same: take `prev[10..26]` as a `Vec<char>` and port the C increment exactly:
   - walk `i` from 15 down while `buf[i] == 'Z'` → set `'0'`;
   - if `i < 0` (all-Z overflow): `thread::sleep(Duration::from_nanos(1_234_567))`
     and retry (loop back to step 1 — recompute timestamp; C recurses, a loop is
     equivalent and avoids unbounded recursion);
   - else bump `buf[i]` to the successor of `buf[i]` in `B32_ALPHABET`
     (C: `strchr(alphabet, buf[i])` then `*(s+1)`);
   - if any char of the random part is not in the alphabet, fall through to
     re-randomization (C `strchr` returns NULL path).
   - Preserve the order: wrap `'Z'`s first, then bump; fall-through last.
5. Else (or fall-through): `let rnd: [u8; 16] = rand::random();`
   map each byte `b` → `B32_ALPHABET[(b % 32) as usize]` into chars 10..26.
   (C `getentropy` + `abort()` on failure; `rand::random` cannot fail — note the
   behavior difference in a comment.)
6. Return the 26-char `String`.

Also implement the `#[cfg(test)]` module in the same file (ports of
`tests/test.c` helpers/tests, see Part B for the shared semantics):
- `is_valid_ulid`: `ulid.len() == 26 && ulid.chars().all(|c| B32_ALPHABET.contains(c))`
- `test_ulid_length`: `ulidgen_fresh().len() == 26`
- `test_ulid_structure`: `is_valid_ulid(&ulidgen_fresh())`
- `test_ulid_uniqueness`: `let a = ulidgen_fresh(); let b = ulidgen(Some(&a)); assert_ne!(a, b)`
- `test_ulid_sortability`: `let a = ulidgen_fresh(); thread::sleep(Duration::from_millis(1)); thread::sleep(Duration::from_micros(500)); let b = ulidgen(Some(&a)); assert!(a < b)`
  (C sleeps 1.5 ms; keep the same duration.)

Gate: `cargo check --all-targets` clean; `cargo test --lib` passes.

### A2. `src/main.rs` (depends on: A1; ports `src/ulidgen.c`)

Implement `main`:

1. Manual arg parsing over `std::env::args().skip(1)` (equivalent of
   `getopt("n:t")`):
   - `-n` consumes the next arg, `parse::<i64>()`; parse failure or missing
     operand → usage on stderr + `process::exit(2)` (C `atol` silently yields 0;
     explicit error is the documented, accepted difference);
   - `-t` sets tag mode;
   - anything else → usage on stderr + `process::exit(2)`.
2. `-t` mode: iterate `io::stdin().lock().lines()`; for each line
   `write!(out, "{} {}\n", ulid, line)` and `out.flush()` after every line
   (line-buffering equivalent of `setvbuf(stdout, 0, _IOLBF, 0)`); thread the
   last ULID: `let ulid = ulidgen(last.as_deref()); last = Some(ulid);`.
3. `-n` mode: loop `n` times, `println!`-style write + newline, threading the
   last ULID the same way.
4. On any write/flush error: message to stderr + `process::exit(1)`
   (equivalent of `exit(!!ferror(stdout))`).
5. Keep the public-domain license header comment.

Gate: `cargo check --all-targets` clean; manual smoke test:
`cargo run -- -n 3` (3 distinct, sorted lines),
`printf 'a\nb\n' | cargo run -- -t` (each line prefixed with `ULID `),
`cargo run -- -n x` exits 2.

## 5. Part B — test files, bottom-up dependency order

### B1. `tests/ulid.rs` (depends on: A1; ports `tests/test.c`)

Fill the five stubs with the same semantics as the lib unit tests, but calling
the public API `ulidgen::ulidgen(None)` / `ulidgen::ulidgen(Some(&prev))` and
`ulidgen::B32_ALPHABET`:

- `is_valid_ulid(ulid: &str) -> bool` — length 26 + alphabet check.
- `test_ulid_length` — `ulidgen(None).len() == 26`.
- `test_ulid_structure` — `is_valid_ulid(&ulidgen(None))` (enabled, unlike C).
- `test_ulid_uniqueness` — two consecutive calls threading `prev` differ.
- `test_ulid_sortability` — generate, sleep 1.5 ms, generate with `prev`,
  assert `first < second`.

Gate: `cargo test` (full suite: lib unit tests + this integration test) passes.

## 6. Final verification

- `cargo test` — all tests green.
- `cargo build --release` — clean build.
- Behavior checklist from design.md §3: 26-char Crockford Base32 output;
  same-ms increment with `'Z'`→`'0'` carry; 1.234567 ms sleep-retry on overflow;
  `-t` preserves lines verbatim after `ULID `; exit 0 on success, >0 on stdout
  write error; license headers preserved.
