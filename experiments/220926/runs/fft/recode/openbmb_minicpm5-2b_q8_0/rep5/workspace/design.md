# FFT Library Translation Design (C → Rust)

## Source Project Overview

A self-contained FFT library implementing Cooley-Tukey radix-2 FFT algorithm. Key features:
- **Two APIs**: `fft()` for out-of-place transforms, `fft_inplace()` for in-place transforms
- **Complex numbers** with float components (real/imag parts)
- **GCC-specific optimizations**: `__builtin_clz`, `_Generic`, `__builtin_expect` for branch prediction
- **Test suite** with known input/output for n=8
- **Static analysis** via assert-based tests

## Target Language: Rust

### Build System

| C (Makefile) | Rust (Cargo.toml + Cargo) |
|---|---|
| Makefile with CC, AR, LDLIBS | Cargo.toml with edition, features |
| `all: libfft.a` | `[[bin]]` or separate crate |
| `test: tests/test.c libfft.a` | `cargo test` |
| `clean` | `cargo clean` |

### Public API Mapping

#### fft_complex_t
```rust
// C: struct { float real; float imag; };
// Rust: struct FftComplex { real: f32, imag: f32 }
pub struct FftComplex {
    pub real: f32,
    pub imag: f32,
}
```

#### FFT_COMPLEX_ADD / SUB / MUL / SELFMUL
```rust
// C: macro-based inline functions
// Rust: inline functions using f32 operations
```

#### fft() - out-of-place
```rust
pub fn fft(x: &[FftComplex], X: &mut [FftComplex], logsize: usize)
```

#### fft_inplace() - in-place
```rust
pub fn fft_inplace(x: &mut [FftComplex], logsize: usize)
```

## Dependency Mapping

| C Library | Rust Equivalent | Notes |
|---|---|---|
| `math.h` (cos, sin, M_PI) | `std::f64::consts::PI` + `math/rust` crate | M_PI is `std::f64::consts::PI`; cos/sin via `math/rust` or `num-complex` |
| `assert.h` | `std::panic::catch_unwind` / `assert!` | Use `assert!` macro |
| `ar` (static library) | N/A (Rust handles binaries differently) | No equivalent; Rust uses different linking model |
| `__builtin_clz` | `builtin_ffs` / bit operations | Need to implement fallback for MSVC compatibility |
| `_Generic` | `#[cfg(target_arch)]` attribute | Architecture-specific code |
| `__builtin_expect` | `unlikely()` / `#[inline]` | Branch hinting via compiler attributes |

## Key Translation Challenges

### 1. Branch Prediction (GCC-specific)
The C code uses `__builtin_expect` to hint branch prediction:
```c
#define likely(expr) __builtin_expect(!!(expr), 1)
#define unlikely(expr) __builtin_expect(!!(expr), 0)
```
In Rust, we use `unlikely()` attribute:
```rust
fn fft_raw(...) { unlikely(logsize == 0); ... }
```

### 2. GCC Builtins (__builtin_clz)
The C code uses `__builtin_clz` which is GCC/Clang specific. 
In Rust, we need to implement a portable version:
```rust
pub fn clz(u32: u32) -> u32 { /* portable implementation */ }
```

### 3. Architecture Compatibility
- GCC `_Generic` → Rust `#[cfg(target_arch)]` or `cfg!()`
- GCC `__builtin_expect` → Rust `unlikely()`
- GCC `_Generic(n, unsigned int: ..., ...)` → Rust `#[cfg(target_arch = "x86")]` etc.

### 4. Static Library vs Binary
C uses `libfft.a` static library. Rust doesn't have static libraries in the same sense;
use separate crates or binaries.

### 5. Test Framework
C uses `assert.h` + manual test runner. Rust uses `cargo test` with `#[test]` attributes.

## Module Structure

```
src/
├── lib.rs          # Public API: FftComplex, fft(), fft_inplace()
├── fft_complex.rs   # FftComplex struct + arithmetic operations
├── fft.rs           # FFT implementation (radix-2)
└── main.rs          # Entry point (optional)
```

## Implementation Plan

### Phase 1: Core Types
- `FftComplex` struct with f32 components
- Arithmetic operations: add, sub, mul, selfmul, copy, swap, setone
- Unit root reciprocal computation

### Phase 2: Bit Twiddling
- Portable `clz()` implementation (works on all platforms)
- `next_reversed_n()` equivalent
- `radder()` and `rader_inplace()` implementations

### Phase 3: FFT Algorithm
- `fft()`: out-of-place transform
- `fft_inplace()`: in-place transform
- Step-based butterfly computation (radix-2)

### Phase 4: Tests
- Test data matching C test suite (n=8, input: alternating ±1,0)
- Verify output matches expected values

### Phase 5: Build System
- `Cargo.toml` with edition 2021
- `src/lib.rs` as library crate
- `src/main.rs` as binary (for testing)
- `Cargo.toml` with `[package]` section

## Risk Assessment

| Risk | Severity | Mitigation |
|---|---|---|
| GCC builtin portability | High | Implement portable clz; test on MSVC too |
| Branch prediction loss | Medium | Use `unlikely()`; document tradeoffs |
| Static library model | Medium | Use separate crate + tests |
| Test data precision | Low | Use f32 throughout; match C test values |
| Missing FFT_COMPLEX_COPY semantics | Low | Implement explicit copy; match C behavior |

## Build Commands

```bash
# Build
cargo build --release

# Run tests
cargo test

# Run specific test
cargo test --test fft_test
```
