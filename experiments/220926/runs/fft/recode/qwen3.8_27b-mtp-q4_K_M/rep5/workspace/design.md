# Translation Design: C FFT library → Rust

## 1. Source project analysis

### 1.1 Structure
```
Makefile          # builds static lib libfft.a from src/fft.c; `make test` compiles tests/test.c and runs it
src/fft.h         # public API + complex-number macros
src/fft.c         # implementation
tests/test.c      # single test: 8-point in-place FFT of [1,-1,1,-1,1,-1,1,-1], asserts real parts
```

### 1.2 Public API (src/fft.h)
- `struct fft_complex { float real; float imag; }` — **single-precision** complex type (`fft_complex_t`).
- `void fft(const fft_complex_t *restrict x, fft_complex_t *restrict X, size_t logsize)` — out-of-place FFT of `2^logsize` points.
- `void fft_inplace(fft_complex_t *x, size_t logsize)` — in-place FFT.
- Macro helpers: `FFT_COMPLEX_ADD/SUB/MUL/SELFMUL/COPY/SWAP/SETONE/UNITROOT_RECIP`.
  - `FFT_COMPLEX_UNITROOT_RECIP(result, N)` = `e^(-i·2π/N)` computed via `cos`/`sin` (double math, stored in float).
  - `FFT_COMPLEX_SELFMUL(self, z)` is a self-aliased multiply (temporaries needed).

### 1.3 Implementation details (src/fft.c)
- `fft_clz(n)`: leading-zero count via `__builtin_clz*` (with portable fallback). Used only by `next_reversed_n`.
- `next_reversed_n(reversed_n, shift)`: bit-reversal successor generator using clz of `~reversed_n`.
- `rader(x, X, logsize)`: out-of-place bit-reversal permutation (copy in reversed order).
- `rader_inplace(x, logsize)`: in-place bit-reversal swaps (skips n=0 and n=size-1).
- `DO_BUTTERFLY(begin, end, step)` macro: one radix-2 DIT stage; unit root `e^(-i·2π/step)`, root squared per inner iteration (`FFT_COMPLEX_SELFMUL`).
- `fft_raw(x, logsize)`: stages with step = 2, 4, 8, …, 2^logsize (early returns for logsize 0/1/2).
- `fft` = `rader` + `fft_raw`; `fft_inplace` = `rader_inplace` + `fft_raw`.
- `likely`/`unlikely` = `__builtin_expect` hints (no semantic effect).
- All arithmetic is `float`; only `cos`/`sin`/`M_PI` come from `<math.h>` (double precision, then truncated to float).

### 1.4 Build/test
- `make` → `libfft.a` (static library). `make test` → compiles `tests/test.c` against the lib and runs it.
- Test: `fft_inplace(data, 3)` on `[1,-1,...]` (8 points); asserts `data[i].real == output[i].real` for all i, where output has `8.0` at index 4 and `0.0` elsewhere. Note: the test only checks the **real** parts and uses exact `==` on floats (works because the values are exactly representable: sums of ±1 and cos(π)=−1, sin(π)≈0 — actually sin(π) in double is ~1.2e-16, truncated to float is ~1.19e-7, not zero; but the test only asserts real parts, and the real parts are exact integers).

## 2. Third-party dependency analysis

The C project has **no third-party dependencies** — only libc (`<math.h>` for `cos`, `sin`, `M_PI`) and compiler builtins (`__builtin_clz*`).

Rust counterparts:
| C dependency | Rust counterpart |
|---|---|
| `<math.h>` `cos`, `sin`, `M_PI` | std `f32::cos`, `f32::sin`, `std::f32::consts::PI` (no crate needed) |
| `__builtin_clz*` | `u64::leading_zeros()` (std, no crate needed) |
| `-lm` | none (libm is built into Rust std) |
| `assert.h` | `assert!` / `assert_eq!` macros (std) |

**Result: zero external crates.** The Rust project can be dependency-free (empty `[dependencies]`), which is the idiomatic choice. (The `num-complex` crate exists but is unnecessary — the source defines its own simple complex struct, and keeping a local `Complex` struct mirrors the source and avoids an extra dependency.)

## 3. Target (Rust) project design

### 3.1 Layout
```
Cargo.toml          # package name e.g. "fft", edition 2021, no dependencies
src/lib.rs          # public API: Complex type, fft(), fft_inplace()
tests/test.rs       # integration test mirroring tests/test.c
```

`Cargo.toml`:
```toml
[package]
name = "fft"
version = "0.1.0"
edition = "2021"

[lib]
name = "fft"
path = "src/lib.rs"
```

### 3.2 API mapping
- `struct fft_complex { float real; float imag; }` →
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq)]
  pub struct Complex { pub real: f32, pub imag: f32 }
  ```
- `void fft(const fft_complex_t *x, fft_complex_t *X, size_t logsize)` →
  ```rust
  pub fn fft(x: &[Complex], X: &mut [Complex], logsize: u32)
  ```
  (slice-based; `x.len()` must equal `1 << logsize`; `X` is written in bit-reversed order then transformed — same as C.)
- `void fft_inplace(fft_complex_t *x, size_t logsize)` →
  ```rust
  pub fn fft_inplace(x: &mut [Complex], logsize: u32)
  ```
- `size_t logsize` → `u32` (log2 of size; sizes up to 2^32 fit; `u32` is the natural choice and matches `1u32 << logsize`).

### 3.3 Internal translation notes
- `fft_clz` → `usize::BITS - n.leading_zeros()` or use `u64::leading_zeros` directly in `next_reversed_n`.
- `next_reversed_n(reversed_n, shift)`:
  ```rust
  fn next_reversed_n(mut reversed_n: usize, shift: u32) -> usize {
      reversed_n <<= shift;
      let count_leading_ones = (!reversed_n).leading_zeros() as u32;
      reversed_n <<= count_leading_ones;
      reversed_n |= 1 << (usize::BITS - 1);
      reversed_n >>= shift + count_leading_ones;
      reversed_n
  }
  ```
  (Note: `~reversed_n` in C is bitwise NOT; in Rust use `!reversed_n`.)
- `rader` / `rader_inplace`: direct port using `std::mem::swap` for `FFT_COMPLEX_SWAP`.
- `DO_BUTTERFLY` macro → a private function `fn butterfly(begin: &mut [Complex], step: usize)` (or inline loop in `fft_raw`). The C macro takes `begin`/`end` pointers and `step`; in Rust iterate over `begin.chunks_mut(step)` or index-based loops.
  - `FFT_COMPLEX_UNITROOT_RECIP(unit, step)` → `Complex { real: (2.0 * PI / step as f32).cos(), imag: -(2.0 * PI / step as f32).sin() }`.
    **Precision note:** C computes `cos(2*M_PI/N)` in **double** then stores into float. To match bit-for-bit, compute in `f64` then cast: `(2.0f64 * std::f64::consts::PI / step as f64).cos() as f32`. This is the safest choice for test compatibility.
  - `FFT_COMPLEX_SELFMUL(root, unit)` → `root = Complex { real: root.real*unit.real - root.imag*unit.imag, imag: root.real*unit.imag + root.imag*unit.real };`
- `fft_raw`: loop `for (let mut step: usize = 8; step <= 1 << logsize; step *= 2)` plus the explicit step=2 and step=4 stages (or just start the loop at step=2 — the C code unrolls the first two stages for performance; a single loop from step=2 is behaviorally identical and simpler).
- `likely`/`unlikely` → drop (Rust has no equivalent; `#[cold]` is not needed).
- `restrict` → Rust's borrow checker enforces this naturally (`x: &[Complex]`, `X: &mut [Complex]` cannot alias).

### 3.4 Test translation (tests/test.rs)
```rust
use fft::{Complex, fft_inplace};

#[test]
fn test_inplace_8() {
    let mut data = [
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        // ... 8 entries total
    ];
    let output = [
        Complex { real: 0.0, imag: 0.0 },
        Complex { real: 0.0, imag: 0.0 },
        Complex { real: 0.0, imag: 0.0 },
        Complex { real: 0.0, imag: 0.0 },
        Complex { real: 8.0, imag: 0.0 },
        Complex { real: 0.0, imag: 0.0 },
        Complex { real: 0.0, imag: 0.0 },
        Complex { real: 0.0, imag: 0.0 },
    ];
    fft_inplace(&mut data, 3);
    for i in 0..data.len() {
        assert_eq!(data[i].real, output[i].real);
    }
}
```
Run with `cargo test`.

Optionally add a test for the out-of-place `fft` API and a round-trip (FFT of a known signal) to improve coverage, since the C test only exercises `fft_inplace` and only asserts real parts.

## 4. Risks

1. **Float precision / exact-equality test.** The C test uses `==` on `f32` real parts. The real parts in this specific test are exact integers (±1, 0, 8) so any correct implementation passes. But if the translator adds extra tests with non-trivial values, exact `==` will be fragile; use `abs(a-b) < 1e-4` style comparisons for any new tests.
2. **Unit-root precision.** C computes the root in double (`cos(2*M_PI/N)`) and truncates to float. If the Rust port computes directly in `f32`, the last ulp may differ. For the given test this doesn't matter (only real parts of exact integers are checked), but to be safe compute in `f64` and cast to `f32`.
3. **`size_t` vs `u32`/`usize`.** The C code uses `size_t` (64-bit on the target). `logsize` is small; `u32` is fine for the API. Internally use `usize` for indices. `1 << logsize` with `logsize` up to 32 fits in `usize` on 64-bit.
4. **Bit-reversal edge cases.** `next_reversed_n` relies on `leading_zeros` of `~n`; ensure the Rust port uses `!n` (bitwise NOT) and that `usize::BITS` matches the C `size_t` width (64 on both, so fine).
5. **`restrict` semantics.** C `restrict` is a hint; Rust borrows make aliasing impossible, so `fft` with `x: &[Complex]` and `X: &mut [Complex]` is safe. No risk, but the API shape changes from pointers to slices (idiomatic).
6. **Macro-based complex ops.** The C header exposes macros so users can swap the complex type. In Rust, expose `Complex` as a public struct with public fields (or methods `add/sub/mul`) so the API remains usable; the macros themselves don't need a direct equivalent.
7. **Coverage.** The C build uses gcov; the Rust equivalent is `cargo-llvm-cov` (optional, not required by `cargo test`). The single C test only covers `fft_inplace`; consider adding an `fft` (out-of-place) test in Rust for parity-plus coverage.

## 5. Summary of mapping

| C | Rust |
|---|---|
| `struct fft_complex` (float real/imag) | `pub struct Complex { pub real: f32, pub imag: f32 }` |
| `fft(x, X, logsize)` | `pub fn fft(x: &[Complex], X: &mut [Complex], logsize: u32)` |
| `fft_inplace(x, logsize)` | `pub fn fft_inplace(x: &mut [Complex], logsize: u32)` |
| `__builtin_clz` | `u64::leading_zeros()` |
| `cos`/`sin`/`M_PI` (double) | `f64::cos`/`f64::sin`/`f64::consts::PI`, cast to `f32` |
| `FFT_COMPLEX_*` macros | inline expressions / `Complex` methods |
| `libfft.a` static lib | `lib` target in `Cargo.toml` |
| `make test` | `cargo test` |
| no third-party deps | no third-party deps |
