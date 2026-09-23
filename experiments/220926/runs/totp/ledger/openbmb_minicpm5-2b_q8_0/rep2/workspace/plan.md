# Rust Translation Plan: C/TOTP → Rust

## Strategy

The C implementation uses SHA1/HMAC-SHA1, HOTP, TOTP, and base32 utilities. We'll translate to idiomatic Rust using the `hmac` and `sha2` crates. Key decisions:

1. **SHA-1**: Use `sha2::Sha1` for SHA-1 hashes; `hmac::Hmac<Sha1>` for HMAC-SHA1.
2. **HOTP/TOTP**: Follow RFC 4226 (TOTP). Time step = 30 seconds, counter offset = 0. Use `hmac::Hmac` with SHA-1 key.
3. **Base32**: Implement from scratch (no external crate needed) or use `base32` crate. We'll implement manually for portability.
4. **Crypto API compatibility**: Provide both raw functions and a `totp` crate-style API.
5. **Testing**: `cargo test` with property-based tests for correctness.

## Architecture

```
src/
├── main.rs          # CLI entry point, demo
├── constants.rs     # TOTP constants (TOKENS_PER_COUNTER, TIME_STEP, etc.)
├── crypto.rs       # SHA-1, HMAC, base32
├── hotp.rs         # HOTP implementation
├── totp.rs         # TOTP implementation
└── errors.rs       # Error types
```

## Seed Tasks

1. **Implement core crypto primitives** (SHA-1, HMAC, base32) — foundation for all other modules
2. **Implement HOTP** — depends on crypto primitives
3. **Implement TOTP** — depends on HOTP
4. **Implement CLI interface** — reads secret/key, QR code generation, verification
5. **Property-based tests** — verify correctness of HOTP/TOTP against RFC 4226
