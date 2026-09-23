# FFT C → Rust Translation Design

## 1. Source project research

### 1.1 Layout

```
c/
├── Makefile          # builds libfft.a from src/fft.c; `make test` compiles tests/test.c
├── src/
│   ├── fft.h         # public API: fft_complex_t, complex-op macros, fft()/fft_inplace()
│   └── fft.c         # implementation: bit-reversal (rader), butterfly (fft_raw)
└── tests/
    ├── test.c        # 8-point FFT of [1,-1,1,-1,1,-1,1,-1], asserts output[4].real == 8.0
    └── test.c_old    # debug variant that prints the array (not part of `make test`)
```

### 1.2 Public interface (src/fft.h)

- `struct fft_complex { float real; float imag; }` — single-precision complex number.
- Macro-based complex ops: `FFT_COMPLEX_ADD/SUB/MUL/SELFMUL/COPY/SWAP/SETONE/UNITROOT_RECIP`.
  All are `do { ... } while (0)` statement macros operating on struct fields.
- `void fft(const fft_complex_t *restrict x, fft_complex_t *restrict X, size_t logsize);`
- `void fft_inplace(fft_complex_t *x, size_t logsize);`
- `logsize` is the **log2** of the transform size; size must be a power of two.

### 1.3 Implementation details (src/fft.c)

- `fft_clz(n)`: count leading zeros. Uses `__builtin_clz/clzl/clzll` via `_Generic` on
  GCC/Clang, with a portable fallback loop. In Rust: `usize::leading_zeros`.
- `next_reversed_n(reversed_n, shift)`: bit-manipulation to produce the next bit-reversed
  index. Uses `~`, shifts, and `INTBITS(size_t)` (= `usize::BITS`).
- `rader(x, X, logsize)`: copies input to output in bit-reversed order (out-of-place).
- `rader_inplace(x, logsize)`: swaps elements in-place to bit-reversed order.
- `DO_BUTTERFLY(begin, end, step)`: macro implementing one radix-2 DIT butterfly stage.
  Uses `FFT_COMPLEX_UNITROOT_RECIP` (cos/sin of 2π/N) for the twiddle factor, then
  `FFT_COMPLEX_SELFMUL` to advance the root.
- `fft_raw(x, logsize)`: runs butterfly stages for step = 2, 4, 8, …, 2^logsize.
- `fft(x, X, logsize)` = `rader(x, X, logsize)` + `fft_raw(X, logsize)`.
- `fft_inplace(x, logsize)` = `rader_inplace(x, logsize)` + `fft_raw(x, logsize)`.
- `likely`/`unlikely` are `__builtin_expect` hints — irrelevant in Rust (compiler decides).

### 1.4 Build & test

- `make` → `libfft.a` (static library).
- `make test` → compiles `tests/test.c` against the library, runs it.
- Test: 8-point FFT of alternating ±1; expects bin 4 (DC of the 2-cycle) = 8.0, rest 0.0.
  Only `real` parts are asserted (imag parts are all 0.0 by symmetry).

## 2. Third-party library analysis

The C project has **zero third-party dependencies** — only the C standard library
(`<math.h>` for `cos`/`sin`, `<limits.h>` for `CHAR_BIT`, `<assert.h>`, `<stdio.h>`).

Rust standard library equivalents:

| C dependency | Rust equivalent | Notes |
|---|---|---|
| `<math.h>` (`cos`, `sin`, `M_PI`) | `std::f32::cos`, `std::f32::sin`, `std::f32::consts::PI` | All in std, no crate needed |
| `<limits.h>` (`CHAR_BIT`) | `usize::BITS` (or `u8::BITS`) | std |
| `<assert.h>` | `assert!` / `assert_eq!` | std |
| `<stdio.h>` (`printf`) | `println!` | std |
| `__builtin_clz` | `usize::leading_zeros()` | std |
| `__builtin_expect` | N/A (no equivalent needed) | |

**No external crates are required.** The `Cargo.toml` should have an empty
`[dependencies]` section.

## 3. Target project design

### 3.1 Crate layout

```
rust/
├── Cargo.toml
├── src/
│   ├── lib.rs          # pub mod fft;
│   └── fft.rs          # FftComplex struct + all FFT functions
└── tests/
    └── test_fft.rs     # integration test (mirrors tests/test.c)
```

The existing skeleton at `rust/` already has this layout with `unimplemented!()`
stubs. The translation fills in the bodies.

### 3.2 Type mapping

| C | Rust |
|---|---|
| `struct fft_complex { float real; float imag; }` | `pub struct FftComplex { pub real: f32, pub imag: f32 }` |
| `size_t` | `usize` |
| `fft_complex_t *restrict` | `&[FftComplex]` / `&mut [FftComplex]` slices |
| `FFT_COMPLEX_ADD(r, a, b)` | `FftComplex::add(&self, other) -> FftComplex` or inline |
| `FFT_COMPLEX_MUL(r, a, b)` | `FftComplex::mul(&self, other) -> FftComplex` |
| `FFT_COMPLEX_SELFMUL(self, z)` | `FftComplex::self_mul(&mut self, z)` |
| `FFT_COMPLEX_SWAP(a, b)` | `std::mem::swap` or `slice.swap(i, j)` |
| `FFT_COMPLEX_UNITROOT_RECIP(r, N)` | `FftComplex::unit_root_recip(n: usize) -> FftComplex` |
| `fft_clz(n)` | `n.leading_zeros() as usize` |
| `INTBITS(size_t)` | `usize::BITS as usize` |

### 3.3 Function mapping

| C function | Rust function | Signature |
|---|---|---|
| `fft_clz(n)` | `fft_clz(n: usize) -> usize` | `n.leading_zeros() as usize` |
| `next_reversed_n(reversed_n, shift)` | `next_reversed_n(reversed_n: usize, shift: usize) -> usize` | direct port of bit ops |
| `rader(x, X, logsize)` | `rader(x: &[FftComplex], X: &mut [FftComplex], logsize: usize)` | copy in bit-reversed order |
| `rader_inplace(x, logsize)` | `rader_inplace(x: &mut [FftComplex], logsize: usize)` | swap in-place |
| `fft_raw(x, logsize)` | `fft_raw(x: &mut [FftComplex], logsize: usize)` | butterfly stages |
| `fft(x, X, logsize)` | `fft(x: &[FftComplex], X: &mut [FftComplex], logsize: usize)` | rader + fft_raw |
| `fft_inplace(x, logsize)` | `fft_inplace(x: &mut [FftComplex], logsize: usize)` | rader_inplace + fft_raw |

### 3.4 Key translation notes

1. **`DO_BUTTERFLY` macro → inline code or helper function.** The C macro expands
   into a loop over butterfly pairs. In Rust, write a `fn butterfly(x: &mut [FftComplex], step: usize)`
   that contains the same logic. The `begin`/`end` pointers become slice bounds.

2. **`restrict` pointers → Rust borrow checker.** `fft` takes `&[FftComplex]` (input)
   and `&mut [FftComplex]` (output) — the compiler enforces no-aliasing.

3. **`FFT_COMPLEX_SELFMUL` aliasing.** In C, `self` and `z` may alias (they don't in
   practice). In Rust, `self_mul(&mut self, z: &FftComplex)` is safe because `z` is
   a shared reference to a different value.

4. **`likely`/`unlikely` → drop.** Rust has no `#[cold]`-equivalent for branch hints
   in stable; the compiler's own heuristics suffice.

5. **`_Generic` for `clz` → `usize::leading_zeros()`.** Rust's `usize` is a single
   type, so no generic dispatch is needed.

6. **`INTBITS(size_t)` → `usize::BITS as usize`.**

7. **Float precision.** The C code uses `float` (f32). Rust must use `f32` throughout
   to match. `cos`/`sin` must be `f32::cos`/`f32::sin`, not `f64`.

8. **Test.** The C test asserts `data[i].real == output[i].real` for all 8 elements.
   The Rust test should do the same with `assert_eq!` on `f32` values. Since the
   expected values are exact (0.0 and 8.0), direct equality is fine — but for
   robustness, a small epsilon comparison could be used. The existing skeleton
   test uses `assert_eq!` which is acceptable for these exact values.

### 3.5 Cargo.toml

```toml
[package]
name = "fft"
version = "0.1.0"
edition = "2021"

[dependencies]
```

No external crates.

### 3.6 Test command

`cargo test` — runs the integration test in `tests/test_fft.rs`.

## 4. Risks

| Risk | Mitigation |
|---|---|
| **f32 vs f64 precision drift.** Using `f64` for `cos`/`sin` then casting would give different results than C's `float` math. | Use `f32::cos`/`f32::sin` and `f32` arithmetic throughout. |
| **Bit-reversal off-by-one.** The `next_reversed_n` logic is subtle. | Port the bit operations verbatim; test with sizes 2, 4, 8, 16. |
| **Butterfly loop bounds.** The C macro uses pointer arithmetic `p != end`. | Use index-based loops over the slice; verify `step` and `half` calculations. |
| **`restrict` semantics.** C allows the compiler to assume `x` and `X` don't overlap. | Rust's type system enforces this via `&` vs `&mut`. |
| **`unimplemented!()` panics.** The skeleton has `unimplemented!()` in every function. | Replace all with real implementations before testing. |
| **`assert_eq!` on f32.** Floating-point equality can be fragile. | For this specific test the values are exact (0.0, 8.0), so `assert_eq!` is safe. For general use, recommend epsilon comparison. |
| **`usize` overflow in bit shifts.** `1 << logsize` can overflow for large `logsize`. | The C code has the same issue; document that `logsize` must be < `usize::BITS`. |
