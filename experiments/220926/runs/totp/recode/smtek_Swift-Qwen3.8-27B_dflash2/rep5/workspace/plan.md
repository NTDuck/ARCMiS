# TOTP (C → Rust) Implementation Plan

Source: dependency-free C TOTP implementation (`totp.{c,h}`, `main.c`,
`test.c`). Target: zero-dependency Rust cargo package `totp` (lib + bin +
integration tests). Test command: `cargo test`.

Skeleton files already exist in the workspace (`Cargo.toml`, `src/lib.rs`,
`src/totp.rs`, `src/main.rs`, `tests/totp.rs`) with stubs (`todo!()`) for
every function. Fill them in the order below.

## Part A — source files (bottom-up dependency order)

1. `src/totp.rs` — the core algorithms. Port from `totp.h` + `totp.c`:
   - `unpack32(x: u32, a: &mut [u8; 4])` — big-endian, from `totp.h` inline.
   - `unpack64(x: u64, a: &mut [u8; 8])` — via two `unpack32` calls.
   - `pack32(a: &[u8; 4]) -> u32` — big-endian.
   - `rotl(x: u32, n: u32) -> u32` — use `x.rotate_left(n)` (guards n=0,
     which is UB in the C original).
   - `sha1(data: &[u8]) -> [u8; 20]` — FIPS 180-3. Allocate a padded buffer
     internally (`new_len = (len+9+63)/64*64`), copy `data`, append `0x80`,
     zero pad, append 64-bit big-endian bit length, then the 80-round
     compression loop with `k[] = {0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC,
     0xCA62C1D6}` and initial `h = {0x67452301, 0xEFCDAB89, 0x98BADCFE,
     0x10325476, 0xC3D2E1F0}`. Keep the algorithm byte-for-byte identical to
     the C code.
   - `hmac_sha1(key: &[u8; 64], data: &[u8]) -> [u8; 20]` — RFC 2104:
     `sha1(key^0x36 ++ data)` then `sha1(key^0x5C ++ inner)`.
   - `hotp(key: &[u8; 64], counter: u64) -> Option<u32>` — RFC 4226:
     `unpack64(counter)`, `hmac_sha1`, dynamic truncation
     `pack32(&hash[hash[19] & 0x0F..]) & 0x7FFF_FFFF`, `% 1_000_000`.
   - `totp(key: &[u8; 64], time: u64) -> Option<u32>` — `hotp(key, time/30)`.
   - `from_base32(s: &str, buf: &mut [u8]) -> Result<usize, ()>` — RFC 4648:
     require `s.len() % 8 == 0` and `buf.len() >= (len+1)/8*5`; decode 8-char
     groups to 5 bytes; accept `=`, A-Z, a-z, 2-7; early-return `i*5+1..4`
     on padding at positions 2/4/5/7; return `Ok(i*5)` at the end.

2. `src/lib.rs` — already written: declares `pub mod totp;`, re-exports all
   public functions, and defines `TOTP_OK`/`TOTP_EBOUNDS` constants. No
   changes needed (verify re-export list matches `src/totp.rs`).

3. `src/main.rs` — CLI, port of `main.c`:
   - `std::env::args().nth(1)`; if `None` → `eprintln!("usage: totp [seed in base32]")`,
     `exit(64)`.
   - Zeroed `[u8; 64]` key; `from_base32(seed, &mut key)`; on `Err` →
     `eprintln!("invalid seed")`, `exit(64)`.
   - `totp(&key, SystemTime::now().duration_since(UNIX_EPOCH).as_secs())` →
     `println!("{:06}", code)` (unwrap; hotp cannot fail for a valid key).

## Part B — test files (bottom-up dependency order)

1. `tests/totp.rs` — port `test.c` 1:1 into `#[test]` functions:
   - `to_hex(a: &[u8]) -> String` helper (lowercase hex, mirrors C `to_hex`).
   - `test_pack` — `unpack32(0x12345678)` → `[0x12,0x34,0x56,0x78]`;
     `unpack64(0x123456789ABCDEF0)` → 8 bytes; `pack32` round-trip.
   - `test_sha1` — empty → `da39a3ee5e6b4b0d3255bfef95601890afd80709`;
     `"abc"` → `a9993e364706816aba3e25717850c26c9cd0d89d`;
     `"The quick brown fox jumps over the lazy dog"` →
     `2fd4e1c67a2d28fced849ee1bb76e7391b93eb12`.
   - `test_hmac_sha1` — RFC 2202: key = 20×0xAA zero-padded to 64, data =
     50×0xDD → `125d7342b9ac11cd91a39af48aa17b4f63f175d3`.
   - `test_hotp` — RFC 4226 Appendix D secret (ASCII "12345678901234567890"
     zero-padded to 64): counters 0/1/2 → 755224/287082/359152.
   - `test_from_base32` — `"MZxw6==="`→3, `"MZxw6YQ="`→4, `"MZxw6YTB"`→5,
     `"MZxw6YTBOI======"`→6; final buffer starts with `"foobar"`.

## Out of scope (per design)

- `std.{c,h}` freestanding shims, `index.html`, WASM/Windows cross builds.

## Verification

`cargo build` (lib + bin) and `cargo test` must pass; the ported vectors
guarantee byte-for-byte algorithm fidelity.
