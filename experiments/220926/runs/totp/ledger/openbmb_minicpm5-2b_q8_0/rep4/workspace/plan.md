# TOTP C-to-Rust Translation Plan

## Overview
Translate a dependency-free C TOTP implementation to Rust with WASM compatibility.

## Source Structure
- `totp.c` - Core algorithms: sha1(), hmac_sha1(), hotp(), totp(), from_base32() + helpers
- `totp.h` - Headers with TOTP_EXPORT attribute, enum (TOTP_OK, TOTP_EBOUNDS), inline defs
- `std.c/std.h` - Freestanding libc stubs (memset, memcpy, strlen)
- `test.c` - Unit tests (FIPS 180-3, RFC 2202, RFC 4226, RFC 6238, from_base32)
- `main.c` - CLI entry point

## Key Translation Challenges
1. **Zero-cost abstractions**: size_t→usize, uint32_t→u32, uint64_t→u64
2. **No std**: Manual string ops for WASM/GBA targets
3. **Bitwise ops**: rotl = rotate-left
4. **Big-endian**: unpack32/unpack64/pack32 for network byte order
5. **Test-driven**: Verify all tests pass
6. **WASM**: Compile for wasm32 target
7. **Performance**: sha1() called frequently - needs efficiency

## Recommended Approach
**Pure Rust with extern "C"** for ABI compatibility + idiomatic Rust
- Use wasm-pack for WebAssembly builds
- Comprehensive tests mirroring C test suite
- Keep dependency-free

## Tasks
1. [ ] Create project structure (Cargo.toml, src/)
2. [ ] Implement core algorithms (sha1, hmac_sha1, hotp, totp, from_base32)
3. [ ] Implement helpers (unpack32, unpack64, pack32, rotl)
4. [ ] Implement freestanding stubs (memset, memcpy, strlen)
5. [ ] Implement tests (mirror test.c)
6. [ ] Implement CLI entry point (main.c)
7. [ ] Build for WASM target
8. [ ] Verify all tests pass
