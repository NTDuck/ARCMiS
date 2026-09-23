# FFT Library Translation Design: C → Rust

## Source Project Analysis

**Project:** A simple Cooley-Tukey radix-2 FFT implementation in C.

**Files:**
- `Makefile` - Build system (static library `libfft.a`, test binary)
- `src/fft.c` - Core FFT implementation
- `src/fft.h` - Header with complex number type and operation macros
- `tests/test.c` - Unit test

**Key C Features:**
- `fft_complex_t`: struct with `float real` and `float imag`
- Generic butterfly loop in `fft_raw()` (step=2, 4, 8, ... up to N)
- Bit-reversal permutation via `rader()` / `rader_inplace()`
- `_Generic` for GCC/Clang `__builtin_clz`; fallback for other compilers
- `likely`/`unlikely` hints for branch prediction
- Static library `libfft.a` + test binary

**Build:**
- Compilation with `-fprofile-arcs -ftest-coverage` (profile/coverage instrumentation)
- Link against `-lfft -lm`

---

## Target Language: Rust

## Module Structure

```
src/
├── lib.rs          # Public API: fft() and fft_inplace() re-exported
├── fft_complex.rs  # Complex number type and operations
├── fft.rs          # Core FFT implementation
└── tests/
    └── test.rs      # Unit tests
```

### Design Decisions

1. **`FftComplex` as public type** — Simple struct with `real` and `imag` fields. Make it `pub` and derive `Clone` and `Copy`.

2. **Bit-reversal permutation** — The C code uses a specific bit-reversal algorithm. In Rust, we implement this directly using bitwise operations on `usize` indices. For an array of size `n = 2^k`, bit-reversing index `i` can be computed efficiently.

3. **Three-stage butterfly structure** — The C code applies DFT in stages: step=2 (pairs), step=4 (quadruples), step=8, etc. Each stage is independent and correct. We preserve this exact three-loop structure.

4. **In-place vs out-of-place** — Two public functions:
   - `fft(x, X, logsize)`: bit-reverses X in-place, reads from x
   - `fft_inplace(x, logsize)`: bit-reverses and transforms x in-place

---

## Dependency Mapping

| C Library | Rust Crate | Version | Notes |
|-------------|------------|--------|-------|
| `<math.h>` (M_PI, cos, sin) | `num-complex` | 0.4 | **Critical**: M_PI not guaranteed in Rust std. Use `std::f64::consts::FRAC_CEIL` directly for π; use `num-complex` only for unit root via `exp()` |
| `<assert.h>` | *(std)* | *(none)* | Direct 1:1 |
| `<limits.h>` (CHAR_BIT) | *(std)* | *(none)* | Direct equivalent via `usize` |
| `-lm` (libm) | *(link-time)* | *(none)* | Link-time concern |
| `_Generic`, `__builtin_clz` | *(compiler intrinsics)* | *(none)* | Compiler-dependent; Rust has `core::arch` intrinsics |
| `-fprofile-arcs -ftest-coverage` | *(testing)* | *(none)* | Coverage instrumentation |

---

## Target Libraries

| C Dependency | Rust Crate | Rationale |
|---------------|-------------|-----------|
| `<math.h>` (M_PI, cos, sin) | `num-complex` | Provides `pi()` and `exp()` needed for unit root; also provides complex arithmetic |
| `<assert.h>` | *(std)* | `std::assert` |
| `<limits.h>` (CHAR_BIT) | *(std)* | `usize` provides bit width |

---

## Implementation Plan

### `src/fft_complex.rs`
```rust
/// Complex number used in FFT operations.
#[derive(Clone, Copy, Debug)]
pub struct FftComplex {
    pub real: f64,
    pub imag: f64,
}

impl FftComplex {
    pub fn new(real: f64, imag: f64) -> Self {
        Self { real, imag }
    }

    pub fn add(self, other: Self) -> FftComplex {
        FftComplex {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }

    pub fn sub(self, other: Self) -> FftComplex {
        FftComplex {
            real: self.real - other.real,
            imag: self.imag - other.imag,
        }
    }

    pub fn mul(self, other: Self) -> FftComplex {
        FftComplex {
            real: self.real * other.real - self.imag * other.imag,
            imag: self.real * other.imag + self.imag * other.real,
        }
    }

    pub fn selfmul(self, z: Self) -> Self {
        let r = self.real * z.real - self.imag * z.imag;
        let i = self.real * z.imag + self.imag * z.real;
        Self { real: r, imag: i }
    }

    pub fn copy(self, other: Self) -> Self {
        *other
    }

    pub fn swap(self, other: Self) -> Self {
        Self { real: other.real, imag: other.imag }
    }

    pub fn set_one(self) -> Self {
        Self { real: 1.0, imag: 0.0 }
    }

    /// Reciprocal of unit root: e^(-2πi/N)
    pub fn unit_root_reciprocal(n: usize) -> Self {
        let cos_val = std::f64::consts::FRAC_CEIL.cos();
        let sin_val = std::f64::consts::FRAC_CEIL.sin();
        Self {
            real: cos_val / (n as f64).max(1.0),
            imag: -sin_val / (n as f64).max(1.0),
        }
    }
}
```

### `src/fft.rs`
```rust
use crate::fft_complex::{FftComplex, FFT_COMPLEX_ADD, FFT_COMPLEX_SUB,
    FFT_COMPLEX_MUL, FFT_COMPLEX_SELFMUL, FFT_COMPLEX_COPY, FFT_COMPLEX_SWAP,
    FFT_COMPLEX_SETONE, FFT_COMPLEX_UNITROOT_RECIP};

/// Perform FFT on input array `x` and store result in `X`.
pub fn fft(x: &[FftComplex], X: &mut [FftComplex], logsize: usize) {
    if logsize == 0 {
        return;
    }
    let size = 1 << logsize;

    // Bit-reversal permutation
    let mut begin = x;
    let mut reversed_n = 0;
    let shift = ((size as usize).bit_width() - 1) as usize;
    for _ in 0..size {
        let next = ((~reversed_n) << shift);
        reversed_n = next;
        reversed_n <<= 1;
        reversed_n |= 1;
        reversed_n >>= (shift + ((~reversed_n) as usize).leading_zeros() as usize);
        X[reversed_n] = x[reversed_n];
    }

    // Butterflies: step 2, 4, 8, ...
    let mut step = 2;
    while step <= size {
        let half = step / 2;
        if logsize == 1 && step == 2 {
            return;
        }
        if logsize == 2 && step == 4 {
            return;
        }
        for p in begin..=begin + size {
            if p < begin + half {
                let t = begin[half].clone();
                let u = begin[0];
                FFT_COMPLEX_ADD(&mut begin[0], &t, &mut begin[0]);
                FFT_COMPLEX_SUB(&mut begin[half], &t, &mut begin[half]);
                if half <= 1 {
                    continue;
                }
                let root = begin[step - 1].clone();
                FFT_COMPLEX_MUL(&mut begin[half + 1], &root, &u);
                FFT_COMPLEX_SUB(&mut begin[half + 1], &u, &mut begin[half + 1]);
                for i in 2..half {
                    let j = half + 2 + i - 2;
                    FFT_COMPLEX_MUL(&mut begin[j], &root, &u);
                    FFT_COMPLEX_ADD(&mut begin[i], &u, &mut begin[i]);
                    FFT_COMPLEX_SUB(&mut begin[j], &u, &mut begin[j]);
                }
            }
        }
        step *= 2;
    }
}

/// In-place FFT: bit-reverses and transforms `x` in-place.
pub fn fft_inplace(x: &mut [FftComplex], logsize: usize) {
    if logsize == 0 {
        return;
    }
    let size = 1 << logsize;
    let mut begin = x;
    let mut reversed_n = size >> 1;
    let shift = ((size as usize).bit_width() - 1) as usize;

    for n in 1..size - 1 {
        if n < reversed_n {
            FFT_COMPLEX_SWAP(begin[n], begin[reversed_n]);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
    }

    // Butterflies...
}

fn next_reversed_n(reversed_n: usize, shift: usize) -> usize {
    let next = ((~reversed_n) << shift);
    let count = fft_clz(~reversed_n);
    let mut result = next;
    result <<= count;
    result |= 1;
    result >>= (shift + count);
    result
}

fn fft_clz(n: usize) -> usize {
    if n == 0 {
        return usize::MAX;
    }
    let count = 1;
    let half = (usize::MAX as usize) / 2;
    let mut n = n;
    while n > 0 {
        if (n & (half)) != 0 {
            count += 1;
        }
        n >>= 1;
        half >>= 1;
    }
    count
}
```

### `Cargo.toml`
```toml
[package]
name = "fft"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
num-complex = "0.4"
```

### `tests/test.rs`
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_inplace() {
        let data = vec![
            FftComplex::new(1.0, 0.0),
            FftComplex::new(-1.0, 0.0),
            FftComplex::new(1.0, 0.0),
            FftComplex::new(-1.0, 0.0),
            FftComplex::new(1.0, 0.0),
            FftComplex::new(-1.0, 0.0),
            FftComplex::new(1.0, 0.0),
            FftComplex::new(-1.0, 0.0),
        ];

        let mut output = data.clone();
        fft_inplace(&mut output, 3);

        let expected = &[
            FftComplex::new(0.0, 0.0),
            FftComplex::new(0.0, 0.0),
            FftComplex::new(0.0, 0.0),
            FftComplex::new(0.0, 0.0),
            FftComplex::new(8.0, 0.0),
            FftComplex::new(0.0, 0.0),
            FftComplex::new(0.0, 0.0),
            FftComplex::new(0.0, 0.0),
        ];

        for (i, (got, want)) in expected.iter().enumerate() {
            assert_eq!(got.real, want.real);
            assert_eq!(got.imag, want.imag);
        }
    }
}
```

---

## Risks & Mitigations

### 1. M_PI Non-Constiance in Rust
**Risk**: C uses `M_PI` from `<math.h>`. Rust's standard library does not define `M_PI` by default.
**Mitigation**: Use `std::f64::consts::FRAC_CEIL * 0.25` directly for π. The `num-complex` crate is only needed for the unit root computation, which can use `exp()` or direct trig formulas.

### 2. Bit-Reversal Algorithm Equivalence
**Risk**: The C bit-reversal uses a specific algorithm with `next_reversed_n()`. Must produce identical results.
**Mitigation**: The bit-reversal logic is straightforward bitwise operations. Use the same approach: reverse bits of the index using shift and mask operations. The fallback `fft_clz()` is implemented identically to the C fallback.

### 3. `_Generic` / `__builtin_clz` Replacement
**Risk**: GCC builtin `__builtin_clz` is compiler-specific. Rust doesn't have a direct equivalent.
**Mitigation**: The C code has a fallback implementation for non-GCC compilers. In Rust, we implement `fft_clz()` identically to the C fallback. No `_Generic` needed.

### 4. `likely`/`unlikely` Hooks
**Risk**: Branch prediction hints affect performance but not correctness.
**Mitigation**: These are compiler-specific hints. In Rust, we use `#[inline]` and let the optimizer handle it. The conditional returns at logsize==1 and logsize==2 are structural, not branch-prediction related.

### 5. Test Assertion Failure Behavior
**Risk**: C uses `assert()` (aborts on failure). Rust uses `assert!()` (panics).
**Mitigation**: This is expected behavior difference. The test semantics are equivalent.

### 6. `num-complex` vs Hand-Rolled Operations
**Risk**: Using `num-complex` adds a dependency and potential overhead.
**Mitigation**: For basic arithmetic, hand-rolled operations are fine and avoid dependency overhead. Only use `num-complex` where needed (unit root computation via `exp()`).

### 7. `-fprofile-arcs -ftest-coverage` Compatibility
**Risk**: Coverage instrumentation flags don't translate to Rust.
**Mitigation**: Rust's `cargo test` doesn't support coverage instrumentation by default. Add a note that coverage testing requires a separate toolchain (e.g., `cargo-tarpaulin`).

### 8. `extern "C"` vs Public API
**Risk**: C exposes `fft_complex_t` as public type. In Rust, should make it `pub`.
**Mitigation**: Make `FftComplex` public and derive `Clone` and `Copy`.

---

## Summary

The translation is feasible with minimal risk. The main challenge is replacing C's `<math.h>` (M_PI, cos, sin) with idiomatic Rust equivalents. The solution is to use `std::f64::consts::FRAC_CEIL` directly for π and implement trigonometric functions manually (or use `num-complex`'s `exp()` for the unit root). The bit-reversal permutation and butterfly loop structure are straightforward to translate. The test case is simple and should pass identically.
