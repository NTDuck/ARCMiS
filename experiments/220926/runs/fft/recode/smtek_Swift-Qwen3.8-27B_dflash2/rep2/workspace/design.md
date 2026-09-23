# FFT Library: C → Rust Translation Design

## 1. Source Project Research

### 1.1 Overview
A small, self-contained radix-2 iterative FFT library in C. It computes the
**forward** DFT (twiddle factor `e^{-i·2π/N}`) of a power-of-two sized complex
sequence. The public API takes `logsize` (log2 of the transform size), not the
size itself.

### 1.2 File layout
```
Makefile          # builds libfft.a from src/fft.c; `make test` builds+runs tests/test.c
src/fft.h         # public header: fft_complex_t type, complex-op macros, API decls
src/fft.c         # implementation: bit-reversal (Rader) + iterative butterflies
tests/test.c      # 8-point transform of [1,-1,1,-1,...], asserts bin 4 == 8.0
```

### 1.3 Public interface (src/fft.h)
- `struct fft_complex { float real; float imag; }` → `fft_complex_t` (single-precision).
- `void fft(const fft_complex_t *restrict x, fft_complex_t *restrict X, size_t logsize);`
  — out-of-place forward FFT.
- `void fft_inplace(fft_complex_t *x, size_t logsize);`
  — in-place forward FFT.
- Complex-op macros: `FFT_COMPLEX_ADD/SUB/MUL/SELFMUL/COPY/SWAP/SETONE/
  UNITROOT_RECIP`. These are implementation details; in Rust they become
  methods on the complex type (or just use `std::f32::mul_add`-free plain ops).

### 1.4 Implementation details (src/fft.c)
- **Bit reversal (Rader)**: `next_reversed_n(reversed_n, shift)` advances the
  bit-reversed index using `clz` of `~reversed_n`. `rader` copies input into
  bit-reversed order (out-of-place); `rader_inplace` swaps pairs in place.
  - `fft_clz` uses `__builtin_clz*` via `_Generic` on GCC/Clang, with a portable
    fallback. In Rust: `usize::leading_zeros()`.
  - `likely`/`unlikely` are `__builtin_expect` hints → Rust `#[cold]`/branch
    hints; not needed for correctness, can be dropped (or use `core::hint`).
- **Butterfly** (`DO_BUTTERFLY` macro): for each `step` (2, 4, 8, …, size),
  walks blocks of length `step`, computes unit root `e^{-i·2π/step}`, and does
  the standard radix-2 butterfly with `half = step/2`.
- **`fft_raw`**: runs butterflies for step = 2, 4, then 8..=size in a loop.
  Early returns for `logsize == 0` (size 1) and `logsize == 1` (size 2).
- Uses `float` (f32) throughout; `cos`/`sin` from `<math.h>` with `M_PI`.

### 1.5 Build & test setup
- `make` → `libfft.a` (static lib) from `fft.o`.
- `make test` → compiles `tests/test.c` against the lib, runs `test_1`.
- Test: 8-point in-place FFT of `[1,-1,1,-1,1,-1,1,-1]`, asserts
  `data[4].real == 8.0` (DC-ish bin; exact in f32 for this input).
- Coverage flags (`-fprofile-arcs -ftest-coverage`) are CI-only; not needed in Rust.

## 2. Third-Party Library Analysis

The C project has **no third-party dependencies** — only libc/libm
(`cos`, `sin`, `M_PI`, `assert`).

| C dependency | Rust counterpart | Notes |
|---|---|---|
| `<math.h>` (`cos`, `sin`, `M_PI`) | `std::f32::cos`, `std::f32::sin`, `std::f32::consts::PI` | Built into std, no crate needed. |
| `<assert.h>` | `assert!` macro / `#[cfg(test)]` | Built in. |
| `<stddef.h>` (`size_t`) | `usize` | Built in. |
| `__builtin_clz*` | `usize::leading_zeros()` | Built in. |
| `__builtin_expect` | `core::hint::cold` / drop | Built in; optional. |

**Conclusion: zero external crates.** The Rust crate depends only on `std`.
This keeps the translation faithful and dependency-free.

## 3. Target Project Design (Rust)

### 3.1 Crate layout
```
Cargo.toml
src/
  lib.rs        # re-exports; crate root
  complex.rs    # FftComplex { real: f32, imag: f32 } + ops
  fft.rs        # bit-reversal + butterfly + public API
tests/
  test.rs       # integration test mirroring tests/test.c
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
No `[dependencies]`.

### 3.2 Type mapping
- `fft_complex_t` → `pub struct FftComplex { pub real: f32, pub imag: f32 }`
  (keep field names `real`/`imag` to mirror the C struct and the test's
  `.real` access).
- `size_t` → `usize`.
- `logsize: size_t` → `logsize: usize`.

### 3.3 API mapping
```rust
// out-of-place forward FFT
pub fn fft(x: &[FftComplex], X: &mut [FftComplex], logsize: usize);
// in-place forward FFT
pub fn fft_inplace(x: &mut [FftComplex], logsize: usize);
```
- Use slices (`&[FftComplex]` / `&mut [FftComplex]`) instead of raw pointers.
  `restrict` is naturally expressed by the borrow checker (disjoint borrows).
- Keep the `logsize` convention (caller passes log2 of size). Document that
  `x.len()` must equal `1 << logsize`.

### 3.4 Complex ops
Replace the macros with inherent methods on `FftComplex` (or free functions):
- `add(a, b) -> FftComplex`, `sub(a, b) -> FftComplex`, `mul(a, b) -> FftComplex`
- `copy` → just `Copy`/`Clone` (derive `Copy, Clone`).
- `swap` → `std::mem::swap` or slice `swap`.
- `set_one() -> FftComplex` (real=1, imag=0).
- `unit_root_recip(n: usize) -> FftComplex` → `real = cos(2π/n)`, `imag = -sin(2π/n)`.
  Use `f32::cos`/`f32::sin` and `std::f32::consts::PI`.

### 3.5 Bit reversal
- `fft_clz(n)` → `n.leading_zeros()`.
- `INTBITS(size_t)` → `usize::BITS`.
- `next_reversed_n(reversed_n, shift)` → same algorithm, `usize` arithmetic.
  Note: the C code relies on wrap-around of `~reversed_n` and shifts; in Rust
  use `wrapping_*` / unchecked shifts carefully. `usize` shifts by >= BITS are
  UB in Rust (panic in debug), so guard: the C `reversed_n << shift` where
  `shift = BITS - logsize` is safe because `reversed_n < 2^logsize`. Keep the
  exact sequence but ensure no shift-by->=BITS. `~reversed_n` is fine (two's
  complement). `reversed_n |= 1 << (BITS-1)` is fine.
- `rader` (out-of-place) and `rader_inplace` → same loops over slices.

### 3.6 Butterfly
- Convert `DO_BUTTERFLY` macro into a generic helper function:
  ```rust
  fn butterfly(x: &mut [FftComplex], step: usize) { ... }
  ```
  operating on the whole slice with the same block/stride logic.
- `fft_raw(x, logsize)`:
  - if `logsize == 0` return;
  - `butterfly(x, 2)`;
  - if `logsize == 1` return;
  - `butterfly(x, 4)`;
  - if `logsize == 2` return;
  - `for step in (8..=(1<<logsize)).step_by(2) { butterfly(x, step); }`
    (C uses `step *= 2` starting at 8 → 8,16,32,…; `step_by(2)` on a range of
    powers is not the same — use an explicit loop `let mut step = 8; while step <= size { butterfly(x, step); step *= 2; }`).

### 3.7 Test translation
`tests/test.rs`:
```rust
use fft::{fft_inplace, FftComplex};

#[test]
fn test_8point() {
    let mut data = [
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
        // ... alternating, 8 total
    ];
    fft_inplace(&mut data, 3);
    assert_eq!(data[4].real, 8.0);
}
```
Run with `cargo test`.

### 3.8 Fidelity / correctness notes
- Keep **f32** (not f64) to match the C `float` and the exact-equality test.
- The test asserts exact `== 8.0`; this holds in f32 for this input because the
  values are exact powers-of-two sums. Preserve f32 to keep the assertion valid.
- Preserve forward-transform sign convention (`e^{-i·2π/N}`).

## 4. Risks of the Translation

1. **Shift-by-≥BITS UB**: Rust panics (debug) / wraps (release) on `usize`
   shifts by ≥ `usize::BITS`, whereas C left-shifts are well-defined modulo
   width for these magnitudes. Must verify every shift in `next_reversed_n`
   stays < BITS. Mitigation: keep `shift = BITS - logsize` and ensure
   `reversed_n < 2^logsize` before shifting; add debug assertions.
2. **`~reversed_n` and leading-ones logic**: relies on two's complement and
   wrap-around; Rust `!` is identical, so low risk, but the `clz` of a value
   that may be 0 must be handled (`leading_zeros` of 0 == BITS, matching C
   `__builtin_clz(0)` being undefined — the C code never passes 0 here, but
   verify).
3. **Exact float equality in test**: must stay f32 and preserve operation
   order to keep `data[4].real == 8.0` exact. Reordering ops (e.g., using
   `mul_add`) could break the assertion.
4. **`restrict` semantics**: Rust borrows enforce disjointness, so the
   out-of-place `fft(x, X, ...)` with `&[T]` and `&mut [T]` is safe; no risk,
   just an API shape change.
5. **`likely`/`unlikely`**: dropping them is fine for correctness; performance
   parity is not a stated goal.
6. **`M_PI` availability**: C defines `USE_MATH_DEFINES`; Rust has
   `std::f32::consts::PI` always — no risk.
7. **API surface**: C exposes macros; Rust exposes methods. Any downstream
   user relying on macros would need updating, but the test only uses the two
   functions, so the public API is just `fft` + `fft_inplace` + the type.

## 5. Build & Test
- Build: `cargo build`
- Test: `cargo test` (matches the required `test_command`).
