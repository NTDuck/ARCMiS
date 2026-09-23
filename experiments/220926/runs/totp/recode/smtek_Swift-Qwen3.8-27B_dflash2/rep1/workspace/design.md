# TOTP (C → Rust) Translation Design

## 1. Source project overview

The source is a small, **dependency-free** educational C implementation of TOTP
(time-based one-time passwords), the algorithm used by authenticator apps.
Author: Sijmen J. Mulder, BSD-2-Clause license.

### Files
| File | Role |
|------|------|
| `totp.h` | Public API + inline helpers (`unpack32/64`, `pack32`, `rotl`). Declares `sha1`, `hmac_sha1`, `hotp`, `totp`, `from_base32`. |
| `totp.c` | The algorithms: SHA-1 (FIPS 180-3), HMAC-SHA1 (RFC 2104), HOTP (RFC 4226), TOTP (RFC 6238), base32 decode (RFC 4648). |
| `main.c` | CLI: `totp <base32-seed>` → prints 6-digit code for current time. |
| `test.c` | Unit tests (assert-based) for pack, sha1, hmac, hotp, base32. |
| `std.c` / `std.h` | Minimal libc shims (`memset`, `memcpy`, `strlen`) for freestanding WASM/GBA builds only. |
| `Makefile` | Builds `totp` (CLI), `test` (test_1), plus cross targets (win32/64, wasm). |
| `index.html` | Frontend for the WASM build (not part of the C logic). |

### Public API (from `totp.h`)
```c
enum { TOTP_OK, TOTP_EBOUNDS };
int    sha1(uint8_t *buf, size_t len, size_t cap, uint8_t hash[20]);
int    hmac_sha1(const uint8_t key[64], const uint8_t *data, size_t len, uint8_t hash[20]);
int    hotp(const uint8_t key[64], uint64_t counter);   // returns code or -1
int    totp(const uint8_t key[64], uint64_t time);      // returns code or -1
size_t from_base32(const char *s, uint8_t *buf, size_t cap); // bytes written, 0 if invalid
```

### Key semantics (verified by compiling & running the C reference)
- **`sha1`** mutates the input buffer in place (padding), requires `cap >= ceil((len+9)/64)*64`. Returns `TOTP_OK`/`TOTP_EBOUNDS`.
- **`hmac_sha1`** takes a fixed 64-byte (zero-padded) key; data ≤ 64 bytes.
- **`hotp`/`totp`** take a fixed 64-byte zero-padded key. `totp(key,t) = hotp(key, t/30)`.
- **`from_base32`**: input length must be a multiple of 8; accepts `A-Z a-z 2-7 =`; returns number of bytes written (handles `=` padding) or `0` on invalid input / insufficient capacity.
- **CLI** (`main.c`): reads one base32 arg, decodes into a 64-byte zero-padded key, prints `totp(key, now)` as `%06d`. Exit code `64` (EX_USAGE) on bad usage/seed.

### Test vectors (all confirmed passing against the C reference)
- SHA1("") = `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- SHA1("abc") = `a9993e364706816aba3e25717850c26c9cd0d89d`
- SHA1("The quick brown fox jumps over the lazy dog") = `2fd4e1c67a2d28fced849ee1bb76e7391b93eb12`
- HMAC-SHA1 (RFC 2202: key=20×0xAA, data=50×0xDD) = `125d7342b9ac11cd91a39af48aa17b4f63f175d3`
- HOTP (secret = ASCII "12345678901234567890"): counter 0→755224, 1→287082, 2→359152
- base32: `MZxw6===`→3, `MZxw6YQ=`→4, `MZxw6YTB`→5, `MZxw6YTBOI======`→6, all decode to "foobar"
- pack/unpack: `unpack32(0x12345678)`→[0x12,0x34,0x56,0x78]; `unpack64(0x123456789ABCDEF0)`→[0x12,0x34,0x56,0x78,0x9A,0xBC,0xDE,0xF0]; `pack32` round-trips.

## 2. Third-party dependency analysis

**The C project has zero third-party dependencies** (only libc). The design
goal is to keep the Rust translation dependency-free as well, so the core
algorithms (SHA-1, HMAC, base32) are re-implemented from scratch, mirroring the
educational intent of the original.

| C dependency | Rust counterpart | Notes |
|--------------|------------------|-------|
| libc (`string.h`, `stdint.h`, `stdio.h`, `time.h`) | Rust std (`std::io`, `std::time`, `std::env`, `std::process`) | No external crate needed. |
| *(none)* | *(none)* | Core crypto re-implemented in-tree. |

Optional (NOT required, kept out to preserve "dependency-free"): `sha1`,
`hmac`, `base32` crates exist, but using them would defeat the educational
purpose and change the code under test. **Decision: no external crates.**

## 3. Target project design (Rust)

### Cargo layout
```
Cargo.toml
src/
  lib.rs        # re-exports public API (sha1, hmac_sha1, hotp, totp, from_base32, TOTP_OK/TOTP_EBOUNDS)
  totp.rs       # the algorithms (port of totp.c)
  main.rs       # CLI (port of main.c)
tests/
  totp.rs       # integration tests (port of test.c)
```
`Cargo.toml`:
```toml
[package]
name = "totp"
version = "0.1.0"
edition = "2021"

[lib]
name = "totp"
path = "src/lib.rs"

[[bin]]
name = "totp"
path = "src/main.rs"
```
No `[dependencies]`.

### API mapping (C → Rust)
Rust has no out-parameters; use owned/`&mut` buffers and `Result`/`Option`.

| C | Rust |
|---|------|
| `enum { TOTP_OK, TOTP_EBOUNDS }` | `pub const TOTP_OK: i32 = 0; pub const TOTP_EBOUNDS: i32 = 1;` (or a `TotpError` enum). Keep the integer constants so the port stays faithful. |
| `int sha1(uint8_t *buf, size_t len, size_t cap, uint8_t hash[20])` | `pub fn sha1(buf: &mut [u8], len: usize, cap: usize, hash: &mut [u8; 20]) -> i32` — mutates `buf` in place like the C version; returns `TOTP_OK`/`TOTP_EBOUNDS`. |
| `int hmac_sha1(const uint8_t key[64], const uint8_t *data, size_t len, uint8_t hash[20])` | `pub fn hmac_sha1(key: &[u8; 64], data: &[u8], len: usize, hash: &mut [u8; 20]) -> i32` |
| `int hotp(const uint8_t key[64], uint64_t counter)` | `pub fn hotp(key: &[u8; 64], counter: u64) -> i32` (returns code, or `-1` on error) |
| `int totp(const uint8_t key[64], uint64_t time)` | `pub fn totp(key: &[u8; 64], time: u64) -> i32` |
| `size_t from_base32(const char *s, uint8_t *buf, size_t cap)` | `pub fn from_base32(s: &str, buf: &mut [u8], cap: usize) -> usize` (bytes written, 0 if invalid) |

Inline helpers (`unpack32/64`, `pack32`, `rotl`) become `#[inline]` private
functions in `totp.rs`.

### Porting notes / idioms
- **SHA-1**: port the FIPS 180-3 loop directly. Use `u32`/`u64` with wrapping
  arithmetic (Rust `u32` `+`/`<<` in release wraps; in debug it panics on
  overflow — use `wrapping_*` or `overflowing`-free `u32` ops via `wrapping_add`
  / `rotate_left`). `rotl(x,n)` → `x.rotate_left(n as u32)`.
  - **Critical**: the C code relies on unsigned wraparound for `T = rotl(a,5)+f+e+k+w`. In Rust use `wrapping_add` (or `u32` arithmetic in a context that wraps). `~b` on `u32` is fine.
- **`unpack64`/`pack32`**: straightforward bit shifts; `pack32` reads 4 bytes big-endian → `u32::from_be_bytes`.
- **`hmac_sha1`**: build a 196-byte scratch buffer, XOR key with 0x36/0x5C, call `sha1` twice.
- **`hotp`**: `unpack64(counter)` → 8 bytes; `hmac_sha1`; dynamic offset `hash[19] & 0xF`; `pack32(...) & 0x7FFFFFFF % 1_000_000`.
- **`from_base32`**: port the char-classification and 8→5 byte packing exactly. Use `s.as_bytes()` and index by `i*8+j`. Preserve the early-return-on-`=` behavior and the multiple-of-8 / capacity checks.
- **CLI (`main.rs`)**: `std::env::args()`, decode seed into a `[u8;64]` (zero-padded), print `format!("{:06}", totp(&key, now))`. `now` from `std::time::SystemTime::now().duration_since(UNIX_EPOCH)`. Exit code 64 on bad usage/seed via `std::process::exit(64)`.
- **`std.c`/`std.h`**: NOT ported — Rust std provides `memset`/`memcpy`/`len` natively; the freestanding shims are irrelevant to the Rust target.

### Tests (`tests/totp.rs`)
Port every assertion from `test.c` into Rust `#[test]` functions:
- `test_pack` — unpack32/unpack64/pack32 (expose helpers as `pub(crate)` or test via public API; simplest: make the inline helpers `pub` in the lib so tests can call them, matching the C test which calls them directly).
- `test_sha1` — the three SHA1 vectors (empty, "abc", fox).
- `test_hmac_sha1` — RFC 2202 vector.
- `test_hotp` — RFC 4226 Appendix D (755224/287082/359152).
- `test_from_base32` — the four base32 cases + "foobar" check.

Test command: `cargo test`.

## 4. Risks of the translation
1. **Integer overflow semantics**: C wraps silently; Rust panics in debug on
   `u32` overflow. Must use `wrapping_add`/`rotate_left` in the SHA-1 core,
   otherwise `cargo test` (debug profile) will panic. Highest-risk item.
2. **In-place buffer mutation**: `sha1` mutates its input buffer and requires
   `cap` headroom. The Rust signature must preserve `&mut [u8]` + `len`/`cap`
   to stay faithful and to keep the test vectors (which pass `buf` with 512
   bytes of headroom) working.
3. **Fixed-size key `[u8; 64]`**: C uses `const uint8_t key[64]`. Rust
   `&[u8; 64]` is a faithful match; the CLI must zero-pad the decoded seed to
   64 bytes exactly as `main.c` does (`memset` then `from_base32`).
4. **`from_base32` edge cases**: the multiple-of-8 length rule, the capacity
   check `(len+1)/8*5`, and the early `=` returns are subtle; port verbatim
   and rely on the four test vectors to catch regressions.
5. **Return-code convention**: `hotp`/`totp` return `-1` on error in C; keep
   `i32` return (not `Option`) to match, since codes are 0..999999 and -1 is
   the sentinel.
6. **`TOTP_EXPORT` visibility attribute**: WASM-only; drop it in Rust (no
   equivalent needed for a normal crate).

## 5. Build / test
- Build: `cargo build`
- Test: `cargo test` (must pass all ported vectors)
- Run CLI: `cargo run -- <base32-seed>`
