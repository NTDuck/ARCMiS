# TOTP Project Translation Design (C → Rust)

## 1. Module Structure

### Source Modules
| Source File | Purpose | Target Equivalent |
|---|---|---|
| `totp.h` | Public API declarations + inline helpers | `src/lib.rs` (pub items) |
| `totp.c` | Core algorithms (SHA1, HMAC-SHA1, HOTP, TOTP, base32) | `src/algorithms.rs` |
| `std.h` / `std.c` | Freestanding libc stubs (memset, memcpy, strlen) | `src/lib.rs` (no_std feature) |
| `main.c` | CLI entry point | `src/cli.rs` |
| `test.c` | Unit tests | `tests/` directory |

### Target Module Structure
```
tokio-totp/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Public API (totp.h → pub items)
│   ├── algorithms.rs   # Core algorithms (totp.c → impl)
│   └── cli.rs          # main.c logic (arg parsing, print → println! / serde)
├── tests/              # test.c → unit tests
├── benches/            # Optional: benchmark harness
├── README.md
├── LICENSE.md
└── design.md
```

### Key Design Decisions
1. **`no_std` feature**: The project supports freestanding targets (WASM, GBA). In Rust, this maps to the `no_std` feature gate. WASM targets will use `wasm-bindgen` for JS interop or keep `no_std` with `alloc` crate.
2. **CLI vs Library separation**: `main.rs` handles CLI args; `lib.rs` exports the algorithm API. This mirrors the C structure where `totp.c` is linked into both `totp` and `test_1`.
3. **Test integration**: Rust's `#[cfg(test)]` attributes in `tests/` directory replaces `test.c`. The test harness runs automatically via `cargo test`.

## 2. Dependency Mapping

| C Dependency | Rust Equivalent | Notes |
|---|---|---|
| `<stdio.h>` | `std::io::Write` / `println!` | Standard library IO |
| `<stdint.h>` | `u8_t`, `u32_t`, `u64_t` (via custom types) | Use `u8`, `u32`, `u64` from `std::num` |
| `<string.h>` | `std::memcpy`, `std::memset`, `strlen` | Directly available |
| `<time.h>` | `std::time::SystemTime::now()` + Unix timestamp | `time(NULL)` → `SystemTime::UNIX` |
| `std.h` (freestanding) | `alloc` crate + manual `memset`/`memcpy`/`strlen` | For `no_std` builds |
| `totp.h` | `pub fn sha1()`, `pub fn hmac_sha1()`, etc. | Public API surface |
| `totp.c` | `impl` block in `algorithms.rs` | Core logic |
| `main.c` | `main()` in `cli.rs` | Entry point |
| `test.c` | `#[cfg(test)]` module | Unit tests |

### Specific API Considerations
- **SHA1**: C uses FIPS 180-3 compliant SHA1 with specific constant ordering and hash values. Rust's `sha2` crate provides a compatible SHA1 implementation but must be verified for exact constant matching. The algorithm steps (padding, message schedule, rounds) must match byte-for-byte.
- **HMAC-SHA1**: C XOR-pads key bytes before/after data, then hashes twice. Rust's `hmac` crate exists but may not match the exact key XOR pattern. Manual implementation recommended to ensure exact match.
- **HOTP/TOTP**: C: HOTP uses HMAC-SHA1 with counter packed as big-endian uint64. TOTP: HOTP with time/counter = time/30. Rust: `hmac_sha1` crate + manual counter packing/unpacking.
- **Base32**: C: RFC 4648 variant, 5-byte output per 8 input chars, stop bit handling. Rust: Write bit-packing logic directly; it's simple enough to avoid test failures.

### WASM Build Configuration
```toml
[package]
name = "totp"
version = "0.1.0"
edition = "2021"

[lib]
name = "totp"
crate-type = ["cdylib", "rlib"]

[package.metadata.wasm-pack.profile.release]
wasm-feature = ["no-entry"]
```

Build with: `wasm-pack build --target web`

## 3. Risk Assessment

### High-Risk Areas

1. **SHA1 constant ordering**: FIPS 180-3 specifies exact constant values and hash initialization. The Rust `sha2` crate may not match byte-for-byte. Need to verify and potentially reimplement with identical constants.

2. **Counter packing/unpacking**: C uses `pack32`/`unpack64` for HOTP counter. Big-endian packing must match exactly. Rust's `u64` packing is `writeln!` style, which may differ from manual bit operations.

3. **Base32 edge cases**: C handles variable-length base32 strings with padding (`=`). Stop bit detection (`s[i*8+2] == '='`) must be carefully translated to handle all RFC 4648 variants.

4. **WASM freestanding target**: The `std.h` stub provides `memset`, `memcpy`, `strlen` for WASM. In Rust, these are either provided by `alloc` crate or need manual implementation. The WASM build requires `-nostdlib` and `-DNO_STD` flags, meaning no libc dependency.

5. **Test coverage**: The C tests cover pack/unpack, SHA1, HMAC-SHA1, HOTP, and base32. All need to be reimplemented in Rust. The test framework changes from `assert!` macros to `#[test]` attributes.

### Mitigation Strategies
- **SHA1**: Reimplement with identical constant arrays and algorithm steps. Use `sha2` crate only if it matches; otherwise write the algorithm inline.
- **Counter packing**: Write explicit `pack64`/`unpack64` functions matching C's behavior exactly.
- **Base32**: Write the bit-packing logic directly; it's simple enough to avoid test failures.
- **WASM**: Use `wasm-bindgen` for JS interop or `no_std` with `alloc` for freestanding WASM.
- **Tests**: Use `#[test]` attributes and `assert_eq!`/`assert!` macros. Run `cargo test` to verify.

## 4. Implementation Plan

### Phase 1: Core Library (`src/lib.rs`)
Implement the public API from `totp.h`:
- `sha1()`: FIPS 180-3 compliant SHA1
- `hmac_sha1()`: RFC 2104 HMAC-SHA1
- `hotp()`: RFC 4226 HOTP
- `totp()`: RFC 6238 TOTP
- `from_base32()`: RFC 4648 base32 decoder

### Phase 2: CLI (`src/cli.rs`)
Parse command-line arguments, call `from_base32()`, compute TOTP, print result.

### Phase 3: Tests (`tests/`)
Reimplement all test cases from `test.c`:
- `test_pack()`: Verify unpack32/unpack64 roundtrips
- `test_sha1()`: Known SHA1 test vectors
- `test_hmac_sha1()`: RFC 2202 key/text vectors
- `test_hotp()`: Appendix D secret
- `test_from_base32()`: Various base32 inputs

### Phase 4: Build System
- `Cargo.toml` with `no_std` feature for WASM
- `Makefile` or `cargo-make` for cross-platform builds
- `wasm-pack.json` for WASM build configuration

## 5. Recommended Rust Crates

| C Library | Rust Crate | Use Case |
|---|---|---|
| None (stdlib) | std | IO, types |
| sha2 | `sha2` | SHA1 (verify compatibility) |
| hmac | `hmac` | HMAC-SHA1 |
| time | `time` | Unix timestamp |
| alloc | `alloc` | no_std WASM support |
| wasm-bindgen | `wasm-bindgen` | WASM JS interop |

**Note**: The project is dependency-free in C. The Rust translation should minimize dependencies. Only use crates when necessary (e.g., `hmac` for HMAC-SHA1). Consider writing HMAC-SHA1 manually to avoid the `hmac` crate dependency.

## 6. Known Differences

1. **Return types**: C uses `int` for error codes and `size_t` for lengths. Rust uses `i32`/`isize` for ints and `usize` for sizes. This is generally compatible.

2. **Memory layout**: C uses `uint8_t` arrays. Rust uses `u8` arrays. Same semantics.

3. **Bit operations**: C uses `rotl` (rotate left). Rust has `rotate_left` in `std::num::TryFromIntegerError` or `bitflags` crate. Use `u32.rotate_left(n)` which is available in Rust 1.75+.

4. **String handling**: C uses C strings (`char*`). Rust uses `String`/`&str`. The `from_base32` function takes a C string pointer; Rust version takes `&str`.

5. **Thread safety**: C functions are not thread-safe (no mutexes). Rust functions will be by default (unsafe by default). Use `unsafe` blocks if needed.

6. **Error handling**: C returns `TOTP_OK` (0) or `TOTP_EBOUNDS` (1). Rust uses `Result<T, Error>` or `Option`. Consider returning `Result` for better ergonomics.
