# ulidgen — C → Rust Translation Design

## 1. Source project research

`ulidgen` is a small public-domain C utility (by Leah Neukirchen, Void Linux) that
generates ULIDs (Universally Unique Lexicographically Sortable Identifiers) or
prefixes stdin lines with them.

### File inventory

| File | Purpose |
|---|---|
| `src/ulid.h` | Single declaration: `void ulidgen_r(char[27]);` |
| `src/ulid.c` | Core ULID generator (`ulidgen_r`) |
| `src/ulidgen.c` | CLI `main`: `-n N` (generate N ULIDs) / `-t` (tag stdin lines) |
| `tests/test.c` | 4 tests: length, structure, uniqueness, sortability (structure test is commented out in `main`) |
| `Makefile` | Builds `ulid.o`, compiles `test_1` from `tests/test.c` + `src/ulid.c`, runs it; install/README targets |
| `README` | Man-page text (ulidgen(1)) |
| `coverage_report.json` | Coverage metadata only (not code) |

### Core algorithm (`src/ulid.c`, `ulidgen_r`)

1. Compute millisecond timestamp: `clock_gettime(CLOCK_REALTIME)` → `tv.tv_sec*1000 + tv.tv_nsec/1000000`.
2. Encode the 48-bit timestamp into the first 10 chars using Crockford Base32
   alphabet `"0123456789ABCDEFGHJKMNPQRSTVWXYZ"`, most-significant digit first
   (`for i in 9..=0: ulid[i] = alphabet[t % 32]; t /= 32`).
3. **Same-millisecond handling**: the function writes into a *caller-reused* 27-byte
   buffer, so it can detect "timestamp part unchanged" by comparing against the
   buffer's previous contents (`same` flag). If unchanged, it increments the random
   part (chars 10..25) in place: walk from index 15 down, wrapping `'Z'` → `'0'`,
   then bump the first non-`'Z'` char to its successor in the alphabet. If the whole
   random part was `'Z'` (overflow), sleep 1.234567 ms (`nanosleep`) and recurse.
   If a char in the random part is not in the alphabet (corrupt buffer), fall
   through to full re-randomization.
4. Otherwise (or on fall-through): fill 16 random bytes via `getentropy`
   (abort on failure) and encode each byte as `alphabet[byte % 32]` into chars 10..25.
5. Buffer is NUL-terminated (`ulid[26] = 0`); output is 26 chars.

### CLI (`src/ulidgen.c`)

- `getopt(argc, argv, "n:t")`: `-n N` (default 1), `-t`.
- `-t` mode: `setvbuf(stdout, 0, _IOLBF, 0)` (line buffering), then
  `getdelim` loop printing `"%s %s"` (ULID, space, line — line keeps its newline).
- `-n` mode: loop `n` times, `puts(ulid)`.
- Exit status: `exit(!!ferror(stdout))` — nonzero if stdout write failed.
- Note: the same `ulid[27]` buffer is reused across iterations, which is what
  enables the same-millisecond increment logic.

### Tests (`tests/test.c`)

- `test_ulid_length`: strlen == 26.
- `test_ulid_structure`: all chars in Crockford Base32 (defined but **not called** in `main`).
- `test_ulid_uniqueness`: two consecutive ULIDs differ.
- `test_ulid_sortability`: after a 1.5 ms `nanosleep`, second ULID sorts after first.

### Build/test setup

`make test` compiles `tests/test.c` + `src/ulid.c` into `test_1` and runs it.
No third-party C dependencies — libc only (`stdint`, `stdlib`, `string`, `time`,
`unistd`, `stdio`).

## 2. Third-party library analysis

The C project has **no third-party dependencies** (libc only). Mapping of the
libc facilities used to Rust counterparts:

| C facility | Rust counterpart | Notes |
|---|---|---|
| `clock_gettime(CLOCK_REALTIME)` | `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` | std, no dep |
| `getentropy(buf, len)` | **`rand` crate** (`rand::random::<[u8;16]>()`) or `getrandom` crate | Rust std has no getentropy; `rand` is the idiomatic choice and uses the OS CSPRNG (getrandom/getentropy) under the hood. `getrandom` is the more direct 1:1 counterpart if minimal deps are preferred. |
| `nanosleep` | `std::thread::sleep(Duration)` | std, no dep |
| `getopt` | manual `std::env::args` parsing (or `clap`) | CLI is tiny (`-n N`, `-t`); manual parsing keeps the project dependency-light. `clap` is the idiomatic alternative if richer CLI is wanted. |
| `getdelim` / `setvbuf(_IOLBF)` | `std::io::BufRead::lines()` on stdin; explicit `flush()` after each printed line | std |
| `abort()` on entropy failure | `panic!` / `std::process::abort()` | preserve fail-hard behavior |
| `exit(!!ferror(stdout))` | check `Write`/`flush` results, `std::process::exit(1)` on error | std |

**Chosen dependency set for the Rust project:** `rand` (v0.9, `rand::random`)
for entropy. Everything else is std. (Alternative: `getrandom` v0.3 directly —
smaller, but `rand` is more idiomatic and future-proof.)

## 3. Target project design (Rust)

### Layout

```
ulidgen/
├── Cargo.toml          # package "ulidgen", lib + bin, edition 2021
├── src/
│   ├── lib.rs          # ulidgen() core + unit tests (ported from tests/test.c)
│   └── main.rs         # CLI: -n N / -t
└── tests/
    └── ulid.rs         # (optional) integration tests mirroring tests/test.c
```

`Cargo.toml`:

```toml
[package]
name = "ulidgen"
version = "0.1.0"
edition = "2021"

[dependencies]
rand = "0.9"
```

### `src/lib.rs` — core API

The C API `ulidgen_r(char[27])` relies on buffer reuse to detect same-millisecond
calls. In Rust the equivalent is an explicit previous-value parameter:

```rust
pub const B32_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID. If `prev` is Some and its timestamp part (first 10 chars)
/// equals the current millisecond, the random part is incremented in place
/// (Crockford-Base32 carry, 'Z' wraps to '0'); on full overflow, sleep ~1.23 ms
/// and retry. Otherwise the random part is 16 fresh CSPRNG bytes.
pub fn ulidgen(prev: Option<&str>) -> String
```

Implementation steps (direct port of `ulid.c`):

1. `let ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;`
2. Encode 10 chars: `for i in (0..10).rev() { push(alphabet[(ms % 32) as usize]); ms /= 32; }`
   (build the 10-char timestamp string; compare with `prev`'s first 10 chars for the
   "same" detection instead of the C buffer-comparison trick).
3. If same as `prev`: take `prev`'s random part (chars 10..26), walk from the end
   (index 15 of the random part) wrapping `'Z'` → `'0'`; if all wrapped (i < 0),
   `thread::sleep(Duration::from_nanos(1_234_567))` and retry (loop, not recursion —
   or keep recursion, it terminates); else bump the char to its successor in the
   alphabet. If any char is not in the alphabet, fall through to re-randomize.
4. Else: `let rnd: [u8; 16] = rand::random();` map each byte to `alphabet[(b % 32) as usize]`.
5. Return the 26-char `String`.

Also expose `pub fn ulidgen_fresh() -> String { ulidgen(None) }` for convenience.

### `src/main.rs` — CLI

```rust
use std::io::{self, Write};
use std::process;

fn main() {
    // parse args: -n N (default 1), -t ; reject unknown flags with usage + exit(2)
    // -t mode: for each line from stdin (BufRead::lines), print "{ulid} {line}\n",
    //          flush after each line (line-buffering equivalent), thread last ULID
    //          through ulidgen(last) for same-ms increment semantics.
    // -n mode: loop n times, println!(ulid), threading last ULID likewise.
    // On any stdout write/flush error: eprintln + process::exit(1)
    // (equivalent of exit(!!ferror(stdout))).
}
```

Argument parsing: iterate `std::env::args().skip(1)`; handle `-n` (take next arg,
`parse::<i64>()`, error-exit on failure), `-t`; anything else → print usage to
stderr, exit 2. (Matches `getopt("n:t")` behavior closely enough; `clap` is an
optional upgrade.)

### Tests (port of `tests/test.c`, run via `cargo test`)

In `src/lib.rs` under `#[cfg(test)]` (and/or `tests/ulid.rs`):

- `test_ulid_length`: `ulid.len() == 26`.
- `test_ulid_structure`: every char in `B32_ALPHABET` (this test was disabled in
  the C `main`; enable it in Rust — it's trivially true and cheap).
- `test_ulid_uniqueness`: two consecutive `ulidgen` calls (threading `prev`) differ.
- `test_ulid_sortability`: generate, `thread::sleep(1.5 ms)`, generate again,
  assert `first < second` lexicographically.

### Behavior-preservation checklist

- 26-char Crockford Base32 output, 48-bit ms timestamp + 80-bit random. ✔
- Same-millisecond uniqueness via in-place increment with 'Z'→'0' carry. ✔
- Overflow → 1.234567 ms sleep + retry. ✔
- Entropy failure → hard abort (C `abort()` → Rust `panic!`/`abort()`). ✔
- `-t` preserves input lines verbatim (including trailing newline) after `ULID `. ✔
- Exit 0 on success, >0 on stdout write error. ✔
- Public domain license header preserved in file comments. ✔

## 4. Risks of the translation

1. **Same-millisecond detection semantics**: C infers "same ms" from buffer
   contents; Rust must thread the previous ULID explicitly. If the caller forgets
   to pass `prev`, same-ms uniqueness degrades to pure randomness (still valid
   ULIDs, just not monotonically incremented). Mitigation: `main.rs` always
   threads it; document the parameter.
2. **Increment edge cases**: the C code's `strchr` + `*(s+1)` bump relies on the
   while-loop having already wrapped `'Z'`s; a direct port must preserve the
   order (wrap first, then bump) and the fall-through re-randomization when a char
   is not in the alphabet. The all-`'Z'` overflow path (sleep + retry) must be
   tested or at least reviewed.
3. **`rand` vs `getentropy`**: `rand::random` is CSPRNG-backed and equivalent in
   quality, but adds a dependency and a version pin (0.9 API: `rand::random::<[u8;16]>()`).
   If the build environment is offline, vendoring or switching to `getrandom`
   (or a `libc::getentropy` FFI shim) may be needed.
4. **Line buffering / flush semantics**: Rust stdout is block-buffered when piped;
   the C `-t` mode is line-buffered. Must `flush()` after each line in `-t` mode to
   avoid output ordering surprises with interactive pipes.
5. **`-n` with non-numeric N**: C `atol` silently yields 0 (prints nothing); Rust
   `parse` errors. Choose explicit error + exit(2) (better UX, slight behavior
   difference) — acceptable.
6. **Test flakiness**: `test_ulid_sortability` depends on a 1.5 ms sleep crossing a
   millisecond boundary — same flake risk as the C test; keep the sleep as-is.
   `test_ulid_uniqueness` is safe because the increment logic guarantees difference
   within the same ms.
7. **`as_millis()` overflow**: `u64` ms since epoch is fine for centuries; the C
   code's `uint64_t` arithmetic matches. No action needed.
