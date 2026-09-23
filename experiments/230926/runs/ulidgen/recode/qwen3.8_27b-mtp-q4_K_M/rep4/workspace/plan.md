# ulidgen — C → Rust Translation Plan

Source: C CLI `ulidgen` (public domain, Leah Neukirchen). Target: Rust crate,
test command `cargo test`. Design details in `design.md`.

## Fragment inventory (source → target)

| Source fragment | File | Target | Target file |
|---|---|---|---|
| `ulidgen_r(char[27])` | `src/ulid.c` (+ decl in `src/ulid.h`) | `pub fn ulidgen_r(ulid: &mut [u8; 27])` | `src/lib.rs` |
| `b32alphabet` (static const) | `src/ulid.c` | `pub const B32_ALPHABET: &[u8; 32]` | `src/lib.rs` |
| `main` (CLI: getopt, tag/count modes) | `src/ulidgen.c` | `fn main() -> std::io::Result<()>` | `src/main.rs` |
| `is_valid_ulid` (helper) | `tests/test.c` | `fn is_valid_ulid(&[u8]) -> bool` | `tests/test.rs` |
| `test_ulid_length` | `tests/test.c` | `#[test] fn ulid_length` | `tests/test.rs` |
| `test_ulid_structure` | `tests/test.c` | `#[test] fn ulid_structure` | `tests/test.rs` |
| `test_ulid_uniqueness` | `tests/test.c` | `#[test] fn ulid_uniqueness` | `tests/test.rs` |
| `test_ulid_sortability` | `tests/test.c` | `#[test] fn ulid_sortability` | `tests/test.rs` |

## Name mapping

| C name | Rust name | Reason |
|---|---|---|
| `ulidgen_r` | `ulidgen_r` | kept — public API fidelity |
| `b32alphabet` | `B32_ALPHABET` | Rust `const` naming convention (UPPER_SNAKE) |
| `main` | `main` | kept |
| `is_valid_ulid` | `is_valid_ulid` | kept |
| `test_ulid_length` / `_structure` / `_uniqueness` / `_sortability` | `ulid_length` / `ulid_structure` / `ulid_uniqueness` / `ulid_sortability` | Rust `#[test]` convention: drop redundant `test_` prefix |
| `tv`, `t`, `same`, `buf`, `rnd`, `n`, `tflag`, `line`, `linelen` | same (lowercase locals) | kept |

> Note: `Cargo.toml` includes an empty `[workspace]` table so the crate stays
> standalone even when nested inside a parent Cargo workspace.

## Part A — source files (bottom-up dependency order)

### A1. `src/lib.rs` — core generator (depends on: `getrandom`, `std::time`, `std::thread`)

Fill in `ulidgen_r(ulid: &mut [u8; 27])` as a faithful port of C `ulidgen_r`:

1. `ulid[26] = 0;` (NUL terminator, C contract).
2. `let mut t: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;`
3. Timestamp loop (least significant 5-bit group at index 9):
   ```rust
   let mut same = true;
   for i in (0..10).rev() {
       let c = B32_ALPHABET[(t % 32) as usize];
       if ulid[i] != c { ulid[i] = c; same = false; }
       t /= 32;
   }
   ```
4. If `same` (same ms as previous call in this buffer): increment the 16-char
   random region `ulid[10..26]` in place:
   - walk `i` from 15 down to 0;
   - `ulid[10+i] == b'Z'` → set `b'0'`, continue;
   - else if the byte is in `B32_ALPHABET` → set to the alphabet successor
     (`B32_ALPHABET[pos + 1]`) and return;
   - if a byte is not in the alphabet → break out and re-randomize;
   - if all 16 were `Z` → `thread::sleep(Duration::from_nanos(1_234_567))`
     and **retry via a `loop`** (re-run the whole function body; do NOT
     recurse — C's recursion is replaced by a loop to avoid stack growth).
5. Otherwise (new ms / fresh buffer / corrupt random part):
   ```rust
   let mut rnd = [0u8; 16];
   getrandom::getrandom(&mut rnd).expect("getentropy failed"); // C: abort()
   for i in 0..16 { ulid[10 + i] = B32_ALPHABET[rnd[i] as usize % 32]; }
   ```

Notes:
- Keep the `&mut [u8; 27]` signature and the compare-against-prior-contents
  "same" logic verbatim — that is the function's only state.
- `getrandom` v0.3 API: `getrandom::getrandom(&mut buf) -> Result<(), Error>`.
- Structure the body so the all-Z retry loops without recursion (e.g. wrap
  the body in a `loop { ... }` with `break` on success, or a small inner
  retry loop around the increment path).

### A2. `src/main.rs` — CLI (depends on: `src/lib.rs`, `std::io`, `std::env`)

Fill in `main() -> std::io::Result<()>`:

1. Manual `getopt("n:t")` equivalent over `std::env::args().skip(1)`:
   - `-n` → next arg must exist and parse as `i64` (default `n = 1`);
     missing value or parse failure → `eprintln!("usage: ulidgen [-n N] [-t]")`
     and `std::process::exit(2)` (stricter than C `atol`, per design §4.5);
   - `-t` → `tflag = true`;
   - anything else → usage error, exit 2.
2. `let mut ulid = [0u8; 27];` (single reused buffer, as in C).
3. `let mut out = std::io::stdout().lock();`
4. Tag mode (`tflag`):
   ```rust
   for line in std::io::stdin().lock().lines() {
       let line = line?;
       ulidgen::ulidgen_r(&mut ulid);
       out.write_all(&ulid[..26])?;   // 26 chars, skip NUL at [26]
       out.write_all(b" ")?;
       out.write_all(line.as_bytes())?;
       out.write_all(b"\n")?;
       out.flush()?;                  // C setvbuf(_IOLBF) equivalent
   }
   ```
5. Count mode: `for _ in 0..n { ulidgen::ulidgen_r(&mut ulid); out.write_all(&ulid[..26])?; out.write_all(b"\n")?; }`
6. Final `out.flush()?; Ok(())` — `io::Result` return gives C's
   `exit(!!ferror(stdout))` semantics (0 on success, non-zero on write error).

## Part B — test files (bottom-up dependency order)

### B1. `tests/test.rs` — integration tests (depends on: `src/lib.rs`)

Fill in the four `#[test]` stubs (helper `is_valid_ulid` is already complete):

1. `ulid_length`: `let mut u = [0u8; 27]; ulidgen_r(&mut u);`
   assert `u[26] == 0` and `is_valid_ulid(&u[..26])` (C: `strlen(ulid) == 26`).
2. `ulid_structure`: fresh buffer, one call, `assert!(is_valid_ulid(&u[..26]))`.
3. `ulid_uniqueness`: two **separate** buffers, two consecutive calls,
   `assert_ne!(&a[..26], &b[..26])`.
4. `ulid_sortability`: call on `a`, `std::thread::sleep(Duration::from_millis(2))`
   (C used 1.5 ms; 2 ms is a safer margin, same intent), call on `b`,
   `assert!(std::str::from_utf8(&a[..26]).unwrap() < std::str::from_utf8(&b[..26]).unwrap())`.

## Verification

- `cargo build` — lib + bin compile.
- `cargo test` — 4 tests pass.
- Smoke: `cargo run -- -n 3` (3 distinct ULIDs); `printf 'a\nb\n' | cargo run -- -t`
  (each line prefixed with a ULID).
