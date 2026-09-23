# TOTP Implementation Translation - Planning Notes

## Current State
- C source code exists for TOTP implementation
- Need to translate to Rust with multi-target support (native, .exe, WebAssembly)

## Delegation Plan
1. **Phase 1**: Project scaffolding (Cargo.toml, workspace structure)
2. **Phase 2**: Core crypto primitives (SHA1, HMAC-SHA1)
3. **Phase 3**: HOTP + TOTP (RFC 4226 + 6238)
4. **Phase 4**: Base32 encoding/decoding
5. **Phase 5**: CLI interface
6. **Phase 6**: Tests
7. **Phase 7**: Build configuration (native, wasm targets)

## Key Decisions
- Use `sha2` crate for SHA1/HMAC-SHA1
- Use `ring` crate for HMAC-SHA1
- Follow FIPS 180-3 compliance
- Keep CLI minimal and matching C version
