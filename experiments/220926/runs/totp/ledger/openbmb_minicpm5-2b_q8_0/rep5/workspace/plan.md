# TOTP Implementation Translation Plan: C → Rust

## Current State
- C implementation of TOTP (RFC 6238) with HOTP (RFC 4226), SHA1, HMAC, Base32
- CLI interface
- Need Rust equivalent with same functionality

## Architecture Decision
- **Rust workspace** with three targets: `native` (x86_64), `wasm32-unknown-unknown`, `windows`
- Core crypto in separate modules
- Shared types across targets

## Module Structure
```
totspec/
├── Cargo.toml (workspace)
├── tospec/src/lib.rs
├── tospec/tests/
│   ├── crypto_tests.rs
│   ├── cli_tests.rs
│   └── integration_tests.rs
├── tospec/tests/crypto_tests.rs
├── tospec/tests/cli_tests.rs
├── tospec/tests/integration_tests.rs
├── tospec/src/
│   ├── main.rs
│   ├── crypto/
│   │   ├── sha1.rs
│   │   ├── hmac.rs
│   │   └── hotp.rs
│   ├── totp.rs
│   ├── encoding.rs
│   └── cli.rs
├── tospec/wasm/ (optional: wasm-pack bindings)
└── tospec/wasm/src/ (optional: wasm bindings)
```

## Key Decisions
1. **SHA1**: Use `sha2` crate (pure Rust, FIPS 180-3 compliant)
2. **HMAC**: Use `hmac` crate from `sha2`
3. **HOTP**: RFC 4226 compliant
4. **TOTP**: RFC 6238 compliant (30s window)
5. **Base32**: Use `base32` crate
6. **CLI**: clap crate for argument parsing
7. **Windows target**: Include `windows` feature flag

## Build Commands
- `cargo build --release` (native)
- `wasm-pack build --target web` (WASM)
- `cargo test` (all tests)

## Testing Strategy
- Unit tests for each crypto primitive
- CLI argument validation tests
- Integration tests for full TOTP workflow
- Deterministic test vectors from RFC 6238 Appendix B

## Files to Create
1. Cargo.toml (workspace manifest)
2. src/main.rs
3. src/crypto/sha1.rs
4. src/crypto/hmac.rs
5. src/crypto/hotp.rs
6. src/totp.rs
7. src/encoding.rs
8. src/cli.rs
9. tests/crypto_tests.rs
10. tests/cli_tests.rs
11. tests/integration_tests.rs

## Files to Delete/Modify
- None (fresh workspace)

## Notes
- Keep C implementation as reference for test vectors
- All crypto must be FIPS 180-3 compliant
- Base32 encoding must match RFC 4648
- CLI interface should mirror C version exactly
