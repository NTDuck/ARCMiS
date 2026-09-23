# TOTP (C → Rust) Translation Design

## 1. Source project overview

The source is a small, dependency-free C implementation of TOTP
(time-based one-time passwords) by Sijmen Mulder (BSD-2-Clause license).
Files:

| File | Role |
|------|------|
| `totp.h` | Public API: `sha1`, `hmac_sha1`, `hotp`, `totp`, `from_base32`, plus inline helpers `unpack32/64`, `pack32`, `rotl`. Status codes `TOTP_OK` / `TOTP_EBOUNDS`. |
| `totp.c` | Algorithm implementations (FIPS 180-3 SHA1, RFC 2104 HMAC-SHA1, RFC 4226 HOTP, RFC 6238 TOTP, RFC 4648 base32 decode). |
| `main.c` | CLI: `totp <base32 seed>` → prints 6-digit code for current time. Exit 64 (EX_USAGE) on bad usage. |
| `test.c` | Unit tests: pack/unpack, SHA1 (3 vectors), HMAC-SHA1 (RFC 2202), HOTP (RFC 4226 Appendix D), base32 decode. |
| `std.{c,h}` | Minimal libc shims (`memset`, `memcpy`, `strlen`) for freestanding WASM/GBA builds (`-DNO_STD`). |
| `index.html` | Web frontend that loads `totp.wasm` and calls `from_base32` + `totp` via raw memory pointers. |
| `Makefile` | Builds `totp` (CLI), `test_1` (tests), Windows cross builds, and a WASM build. |

Key API semantics to preserve:

- `sha1(buf, len, cap, hash)`: **clobbers** `buf` in place (padding), requires
  `cap >= len + 9 + 63` rounded up to a 64-byte multiple. Returns `TOTP_OK`
  (0) or `TOTP_EBOUNDS` (1).
- `hmac_sha1(key[64], data, len, hash)`: key is a fixed 64-byte zero-padded
  buffer; `len` must be ≤ 64.
- `hotp(key[64], counter) -> i32`: 6-digit code, `-1` on error.
- `totp(key[64], time) -> i32`: `hotp(key, time/30)`.
- `from_base32(s, buf, cap) -> size_t`: returns bytes written, `0` on invalid
  input. Input length must be a multiple of 8; accepts `=` padding, upper/lower
  case, digits 2–7.

## 2. Third-party dependency analysis

The C project is explicitly **dependency-free** (no external libraries, only
libc). Therefore the Rust translation should also use **zero external
crates** — the whole point of the project is an educational from-scratch
implementation.

| C dependency | Rust counterpart | Notes |
|---|---|---|
| libc (`string.h`, `stdint.h`, `time.h`) | Rust std (`std::time`, `std::mem`, `std::str`) | Built-in; no crate needed. |
| (none) | (none) | No `sha1`/`hmac`/`base32` crates — implement by hand to match the educational intent. |

Test command is `cargo test`, which uses the built-in `#[test]` harness — no
test framework crate required.

## 3. Target project design (Rust)

### 3.1 Crate layout

```
Cargo.toml
src/
  lib.rs        # re-exports, TOTP_OK/TOTP_EBOUNDS constants
  totp.rs       # sha1, hmac_sha1, hotp, totp, from_base32 + helpers
  main.rs       # CLI (binary target `totp`)
tests/
  totp.rs       # integration tests mirroring test.c
```

- Package name: `totp`. Binary target `totp` from `src/main.rs`.
- `src/lib.rs` exposes the public API; `src/main.rs` is a thin CLI.

### 3.2 API mapping (C → Rust)

Rust idioms replace C's out-parameters and status codes:

```rust
pub const TOTP_OK: i32 = 0;
pub const TOTP_EBOUNDS: i32 = 1;

// Helpers (pub(crate) or pub, mirroring the inline functions in totp.h)
pub fn unpack32(x: u32, a: &mut [u8; 4]);
pub fn unpack64(x: u64, a: &mut [u8; 8]);
pub fn pack32(a: &[u8; 4]) -> u32;
pub fn rotl(x: u32, n: u32) -> u32;

// SHA1: C clobbers buf in place; in Rust, take the message as a slice and
// return the 20-byte digest. This is cleaner and avoids the cap parameter.
// Keep a signature-compatible variant if desired, but the idiomatic form:
pub fn sha1(data: &[u8]) -> [u8; 20];

// HMAC-SHA1: key is a 64-byte zero-padded buffer in C.
pub fn hmac_sha1(key: &[u8; 64], data: &[u8]) -> [u8; 20];

// HOTP / TOTP: return Option<u32> (None on error) or keep i32 with -1.
// Idiomatic: Option<u32>.
pub fn hotp(key: &[u8; 64], counter: u64) -> Option<u32>;
pub fn totp(key: &[u8; 64], time: u64) -> Option<u32>;

// Base32: return Ok(len) / Err, or Option<usize>.
pub fn from_base32(s: &str, buf: &mut [u8]) -> Result<usize, ()>;
```

Design decisions:

1. **SHA1 signature**: The C `sha1(buf, len, cap, hash)` mutates the input
   buffer for padding. In Rust, allocate a padded buffer internally and take
   `&[u8]` → `[u8; 20]`. This preserves the algorithm (FIPS 180-3) exactly
   while being idiomatic. The `TOTP_EBOUNDS` overflow check on `len` is
   unreachable for `&[u8]` (length is bounded), so it can be dropped or kept
   as a defensive check.
2. **Error handling**: C returns `TOTP_OK`/`TOTP_EBOUNDS` and `-1`. Rust
   idiomatic: `Option<u32>` for hotp/totp, `Result<usize, ()>` for
   `from_base32`. Keep the `TOTP_OK`/`TOTP_EBOUNDS` constants exported for
   fidelity if desired, but the primary API uses Option/Result.
3. **Key type**: C uses fixed `uint8_t key[64]`. Rust: `&[u8; 64]`.
4. **`from_base32`**: C writes into a caller buffer and returns byte count.
   Rust: write into `&mut [u8]` and return `Result<usize, ()>` (bytes written).
   Preserve the exact decoding logic (8-char groups, `=` padding, case
   insensitive, digits 2–7) and the early-return byte counts
   (`i*5+1..4` on padding).

### 3.3 CLI (`src/main.rs`)

Mirror `main.c`:

- `args().nth(1)`; if missing → `eprintln!("usage: totp [seed in base32]")`,
  `exit(64)`.
- `from_base32` into a 64-byte zeroed key; on error → `eprintln!("invalid seed")`,
  `exit(64)`.
- `totp(&key, SystemTime::now().duration_since(UNIX_EPOCH).as_secs())` →
  `println!("{:06}", code)`.

### 3.4 Tests (`tests/totp.rs`)

Port `test.c` 1:1 into `#[test]` functions:

- `test_pack`: `unpack32(0x12345678)`, `unpack64(0x123456789ABCDEF0)`, `pack32`.
- `test_sha1`: empty → `da39a3ee...`, `"abc"` → `a9993e36...`,
  `"The quick brown fox..."` → `2fd4e1c6...`.
- `test_hmac_sha1`: RFC 2202 (key 20×0xAA, data 50×0xDD) →
  `125d7342b9ac11cd91a39af48aa17b4f63f175d3`.
- `test_hotp`: RFC 4226 Appendix D secret, counters 0/1/2 → 755224/287082/359152.
- `test_from_base32`: `"MZxw6==="`→3, `"MZxw6YQ="`→4, `"MZxw6YTB"`→5,
  `"MZxw6YTBOI======"`→6, and `"foobar"` content check.

Use a `to_hex` helper mirroring the C one.

### 3.5 What is NOT translated

- `std.{c,h}`: the freestanding libc shims are unnecessary in Rust (std is
  always available; no WASM freestanding target is a stated goal).
- `index.html` / WASM build: out of scope for `cargo test`; the core library
  is what matters. (Could be a future `wasm32` target, but not required.)
- Windows cross builds: `cargo build --target x86_64-pc-windows-msvc` works
  out of the box; no design needed.

## 4. Risks

1. **SHA1 in-place mutation semantics**: The C API clobbers the input buffer.
   The Rust redesign to `&[u8] → [u8; 20]` changes the calling convention.
   This is intentional (idiomatic) and safe, but any code expecting the C
   signature would need adaptation. Mitigation: keep the algorithm byte-for-byte
   identical; tests verify digests.
2. **Overflow / bounds checks**: C checks `len > SIZE_MAX-9-63` and
   `new_len > cap`. In Rust, `&[u8]` length is bounded by `usize`, so these
   are largely unreachable. The padding allocation `len + 9 + 63` rounded up
   must use checked arithmetic to avoid panic on huge inputs (use
   `checked_add` / `checked_mul` and return an error, mirroring
   `TOTP_EBOUNDS`).
3. **`rotl` with n=0**: C `rotl(x, 0)` = `x << 0 | x >> 32` — `x >> 32` on a
   32-bit value is UB in C. In Rust, `x >> (32 - n)` with `n=0` is `x >> 32`,
   which panics in debug / is masked in release. The C code only calls
   `rotl` with n=1 and n=30, so it's safe, but the Rust helper should guard
   (e.g., `n %= 32` or use `rotate_left`). Use `u32::rotate_left` for safety.
4. **`from_base32` edge cases**: The C code has subtle behavior — it returns
   early on padding and requires length multiple of 8. Must replicate exactly,
   including the `cap` check `(strlen(s)+1)/8*5`. Tests cover the main cases.
5. **`hotp` truncation**: `pack32(&hash[hash[19] & 0xF]) & 0x7FFFFFFF` —
   index into the 20-byte hash with offset 0..15, read 4 bytes, mask top bit.
   Must ensure the slice bounds are valid (offset ≤ 15, so `&hash[off..off+4]`
   is always in range). Straightforward port.
6. **Exit code 64**: Preserve `EX_USAGE` = 64 in the CLI.

## 5. Build / test

- `Cargo.toml`: `[package] name = "totp"`, edition 2021, no dependencies.
- `cargo test` runs `tests/totp.rs` (and any unit tests in `src/`).
- `cargo build` produces the `totp` binary.
