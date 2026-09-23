# TOTP Rust Translation Design Document

## Overview

This document describes the translation of a C-based TOTP (Time-based One-Time Password) implementation into Rust. The original is a "from-scratch, dependency-free" educational project implementing SHA-1, HMAC-SHA1, HOTP, and TOTP algorithms per FIPS 180-3 and RFC specifications.

## Source Project Analysis

### Source Files
- **totp.h**: Public API declarations (5 functions)
- **totp.c**: Core algorithm implementations (SHA-1, HMAC-SHA1, HOTP, TOTP, base32 decoder)
- **std.h/std.c**: Minimal freestanding libc definitions (memset, memcpy, strlen)
- **test.c**: Unit tests (pack/unpack, SHA-1, HMAC-SHA1, HOTP, base32)
- **main.c**: CLI entry point
- **Makefile**: Cross-compilation build system (Windows x86/x64, WASM)

### Key Algorithms Implemented
1. **SHA-1** (FIPS 180-3): Standard hash function with 80-round compression
2. **HMAC-SHA1** (RFC 2104): Keyed message authentication code
3. **HOTP** (RFC 4226): Counter-based one-time password generator
4. **TOTP** (RFC 6238): Time-windowed HOTP generator
5. **Base32 Decoder** (RFC 4648): Base32 string to byte array conversion

### Build System
- Cross-compilation via Makefile (hosted vs freestanding)
- WASM target using `-nostdlib -fvisibility=hidden`
- No external dependencies (self-contained)

## Target Language: Rust

### Why Rust?
- Zero-cost abstractions, safe concurrency
- Rich standard library for memory management (`Box`, `Vec`, `memchr`)
- Excellent ecosystem for cryptography (`ring`, `ring-rs`)
- Native cross-compilation support
- Idiomatic approach to this kind of algorithmic implementation

## Target Module Structure

```
src/
├── lib.rs          # Public API re-export
├── totp.rs          # Core TOTP algorithms
├── crypto/
│   ├── sha1.rs     # FIPS 180-3 SHA-1
│   └── hmac.rs    # HMAC-SHA1
├── hotp.rs         # RFC 4226 HOTP
├── totp.rs         # RFC 6238 TOTP
├── base32.rs      # RFC 4648 base32 decoder
└── error.rs       # Error types (TOTP_OK, TOTP_EBOUNDS)

tests/
├── integration.rs # Integration tests
└── unit.rs        # Unit tests

Cargo.toml
```

### Dependency Mapping

| C Library | Rust Equivalent | Version Notes |
|-----------|---------------|--------------|
| `<stddef.h>` | `std::size_t` / `usize` | Same concept, Rust uses `usize` |
| `<stdint.h>` | `u8`, `u32`, `u64` | Direct Rust types |
| `<string.h>` | `std::mem::memset` / `memchr` / `str::len` | `memchr` preferred for performance |
| `<time.h>` | `std::time::SystemTime::now()` | Returns `SystemTime` struct |
| `<stddef.h>` SIZE_MAX | `usize::MAX` | Same concept |
| External crypto | `ring` crate | Recommended for cryptographic operations |

### Key Design Decisions

1. **SHA-1 Implementation**: Use `ring` crate's `sha1` module which implements FIPS 180-3 compliant SHA-1
2. **HMAC-SHA1**: Use `ring::hmac` module which provides HMAC-SHA1
3. **HOTP/TOTP**: Implement using `ring`'s crypto primitives
4. **Base32**: Use `ring::base32` or implement manually (it's simple enough)
5. **Error handling**: Return `Result<T, Error>` instead of C's `int` return codes

## Build System Translation

### Makefile → Cargo.toml + build.rs
- `cargo build --release` replaces Makefile builds
- Cross-compilation via `cargo +nightly-unknown-target` or `rustup target add` + `cargo build --target wasm32-unknown-unknown`
- Windows targets via `cargo install wasm-pack` for native binaries

## Risk Assessment

### High Risk
1. **SHA-1 security**: SHA-1 is cryptographically broken (collision attacks). The C implementation is a direct translation, so it will have the same vulnerability. This is not a design choice — it's a constraint of the algorithm being implemented.
2. **Bit rotation (`rotl`)**: C uses inline assembly for bit rotation. In Rust, this is just `rotate_left()` from `std::num::TryFromIntegerError` or manual bitwise operations. No special handling needed.
3. **WASM freestanding mode**: The C code uses `-nostdlib` for WASM. Rust's `#[cfg(target_arch = "wasm32")]` feature flags can handle this, but we need to ensure no_std constraints are respected.

### Medium Risk
1. **Memory layout differences**: C uses `uint8_t *` pointers; Rust uses `Vec<u8>` or `&[u8]`. Need careful conversion.
2. **Counter overflow**: HOTP uses `uint64_t` counter; Rust's `u64` maps directly.
3. **Time handling**: C uses `time(NULL)` (Unix timestamp); Rust needs `SystemTime::now() - Duration::seconds(30)` for TOTP window.

### Low Risk
1. **Base32 decoding**: Straightforward mapping, well-defined spec.
2. **Test coverage**: Original has comprehensive tests; Rust's `#[test]` attribute provides equivalent coverage.
3. **CLI interface**: argparse-like in C vs clap in Rust.

## Implementation Strategy

### Phase 1: Core Cryptography
- Start with `ring` crate for SHA-1 and HMAC-SHA1
- Implement `sha1()` and `hmac_sha1()` matching C signatures exactly
- Add `unpack32()` and `pack32()` helpers

### Phase 2: HOTP/TOTP
- Implement HOTP using `ring::hmac` with counter encoding
- Implement TOTP with time-window logic (30-second default)

### Phase 3: Base32 Decoder
- Implement RFC 4648 base32 decoder
- Handle padding characters

### Phase 4: CLI
- Use `clap` crate for argument parsing (replaces manual argc/argv handling)
- Use `serde` for base32 string input

### Phase 5: Tests
- Mirror the original test.c test cases
- Use `#[test]` attributes in Rust

## Build Targets

| Target | Method |
|--------|---------|
| Linux x86_64 | `cargo build --release` |
| Windows x86_64 | `cargo install wasm32-unknown-unknown` + `wasm-pack build --target web` |
| Windows x86_64 native | `cargo install wasm32-unknown-unknown` + `wasm-pack build --target wasm32-unknown-unknown --out dir` |
| WASM | `wasm-pack build --target web` |

## Summary

This is a **faithful translation** of a cryptographic algorithm implementation from C to Rust. The core challenge is maintaining cryptographic correctness while adapting to Rust's type system and safety guarantees. The `ring` crate provides a mature, well-tested cryptographic library that can serve as a drop-in replacement for the C SHA-1/HMAC-SHA1 implementations. The main risks are around SHA-1's known cryptographic weakness (which the C code inherits) and ensuring the WASM target compiles correctly with the right feature flags.
