# Source Project Analysis

## Files Found:
1. LICENSE.md - MIT-style license
2. README.md - Project documentation  
3. main.c - CLI entry point
4. std.h - freestanding libc definitions (memset, memcpy, strlen)
5. std.c - Implementation of std.h functions
6. totp.h - Public API declarations
7. totp.c - Core algorithm implementations
8. test.c - Unit tests

## Key Components:

### totp.h (Public API):
- `sha1()`: FIPS 180-3 SHA-1, takes buffer+len+cap, returns 20-byte hash
- `hmac_sha1()`: RFC 2104 HMAC-SHA1, key+data+len → 20-byte hash
- `hotp()`: RFC 4226 HOTP, key+counter → code (returns int)
- `totp()`: RFC 6238 TOTP, key+time → code (calls hotp internally)
- `from_base32()`: RFC 4648 base32 decoder, string→bytes

### totp.c Implementation:
- Pure C, no external dependencies (std.h provides minimal freestanding functions)
- Uses inline assembly for `rotl` (bit rotation)
- Static constants for SHA-1 K values
- All functions are `TOTP_EXPORT` marked with visibility attribute

### Build System:
- Makefile with cross-compilation targets (Windows x86/x64, WASM)
- Default flags: -Wall -Wextra
- WASM: -nostdlib -fvisibility=hidden -DNO_STD, linker flags for WebAssembly

### Test.c:
- Tests for pack/unpack32, sha1 (known test vectors), hmac_sha1, hotp, from_base32
- Uses assert.h and snprintf

### std.h/std.c:
- Minimal freestanding implementations of memset, memcpy, strlen
- Used by both hosted and freestanding targets

## Architecture:
- Single source file approach (totp.c contains everything)
- No external libraries, self-contained algorithms
- Designed for educational purposes ("from-scratch, dependency-free")
