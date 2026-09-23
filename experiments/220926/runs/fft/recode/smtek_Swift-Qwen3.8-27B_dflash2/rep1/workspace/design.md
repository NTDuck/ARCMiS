# Translation Design: C FFT library → Rust

## 1. Source project research

### Layout
```
Makefile
src/fft.h      # public header: fft_complex_t + complex-op macros + 2 function decls
src/fft.c      # implementation: bit-reversal (Rader) + iterative radix-2 butterflies
tests/test.c   # single test: 8-point in-place FFT of [1,-1,1,-1,1,-1,1,-1]
```

### Build/test setup
- `make` builds `libfft.a` (static archive) from `src/fft.c` with `-O3 -g -fPIC` and coverage flags.
- `make test` compiles `tests/test.c` against the archive and runs it.
- Linker needs `-lm` (cos/sin).

### Public interface (src/fft.h)
- `struct fft_complex { float real; float imag; }` — **single-precision (f32)** complex number, typedef `fft_complex_t`.
- `void fft(const fft_complex_t *restrict x, fft_complex_t *restrict X, size_t logsize);`
  Out-of-place forward FFT of `2^logsize` samples.
- `void fft_inplace(fft_complex_t *x, size_t logsize);`
  In-place forward FFT of `2^logsize` samples.
- A family of macros implementing complex arithmetic on the struct:
  `FFT_COMPLEX_ADD/SUB/MUL/SELFMUL/COPY/SWAP/SETONE/UNITROOT_RECIP`.
  `UNITROOT_RECIP(result, N)` sets `result = e^{-i·2π/N}` using `cos`/`sin`
  (note: computed with `float`-precision `M_PI` in the C code).

### Implementation details (src/fft.c)
- `fft_clz(n)`: count-leading-zeros via `__builtin_clz*` (GCC/Clang) with a
  portable fallback. In Rust: `usize::leading_zeros`.
- `next_reversed_n(reversed_n, shift)`: O(1) bit-reversal successor using
  clz of `~reversed_n`. Pure integer bit-twiddling on `size_t`; translates
  directly to `usize` ops.
- `rader(x, X, logsize)`: out-of-place bit-reversal permutation (copies
  `x[n]` to `X[reversed_n]`).
- `rader_inplace(x, logsize)`: in-place bit-reversal swaps; loop runs
  `n = 1 .. size-1` (note: `size - 1` underflows if `size == 0`, i.e.
  `logsize == 0` — the C code is only safe for `logsize >= 1` in the
  in-place path; `fft_raw` handles `logsize == 0` by returning early, but
  `rader_inplace` is called first. In practice the test uses `logsize = 3`.)
- `DO_BUTTERFLY(begin, end, step)` macro: one stage of the iterative
  radix-2 DIT FFT. For each block of `step` elements:
  - butterfly on (0, half) with no twiddle,
  - butterfly on (1, half+1) with twiddle `unit`,
  - then for `i = 2..half`: twiddle `root *= unit` (self-mul), butterfly on
    (i, half+i).
  Twiddle `unit = e^{-i·2π/step}` (reciprocal unit root).
- `fft_raw(x, logsize)`: runs butterfly stages for `step = 2, 4, 8, ..., 2^logsize`.
- `fft` = `rader` + `fft_raw`; `fft_inplace` = `rader_inplace` + `fft_raw`.

### Test (tests/test.c)
- 8-element array `[1,-1,1,-1,1,-1,1,-1]` (real parts), `fft_inplace(data, 3)`.
- Asserts each `data[i].real == output[i].real` where output is
  `[0,0,0,0,8,0,0,0]`. Exact float equality is fine here because the values
  are exact powers-of-two sums of exact cos/sin results at these angles
  (cos(π/4) etc. are not exact, but the imaginary parts cancel and the real
  sums land exactly on 0 and 8 in f32 for this specific input — the C test
  relies on this and passes).

## 2. Third-party library analysis

The C project has **no third-party dependencies** — only libc/libm
(`cos`, `sin`, `M_PI`).

| C dependency | Rust counterpart | Notes |
|---|---|---|
| libm (`cos`, `sin`, `M_PI`) | Rust std: `f32::cos`, `f32::sin`, `std::f32::consts::PI` | No crate needed. |
| `__builtin_clz*` | `usize::leading_zeros` | std, no crate. |
| `assert.h` | `assert!` / `assert_eq!` | std. |
| `stdio.h` (`printf`) | `println!` | std. |

**Result: zero external crates.** The Rust project needs no `[dependencies]`
entries. (Optionally `num-complex` could be used, but the source uses a
plain f32 pair struct and hand-rolled arithmetic; keeping a local
`FftComplex` struct is the faithful, dependency-free translation.)

## 3. Target project design (Rust)

### Cargo project layout
```
Cargo.toml          # package name "fft", edition 2021, no dependencies
src/lib.rs          # public API: FftComplex, fft, fft_inplace
src/fft.rs          # (optional split) implementation; or keep all in lib.rs
tests/test.rs       # integration test mirroring tests/test.c
```

Simplest faithful layout: everything in `src/lib.rs` (the C project is one
translation unit), plus `tests/test.rs` for the integration test.

### Cargo.toml
```toml
[package]
name = "fft"
version = "0.1.0"
edition = "2021"

[lib]
name = "fft"
path = "src/lib.rs"
```
No dependencies. `cargo test` builds the lib and runs `tests/test.rs`.

### Type mapping
- `fft_complex_t` → `pub struct FftComplex { pub real: f32, pub imag: f32 }`
  (keep f32 to match the C `float` fields exactly; do **not** upgrade to f64).
- `size_t` → `usize`.
- `logsize` parameter stays `usize`.
- Complex macros → inherent methods on `FftComplex`:
  - `add(a, b) -> FftComplex`, `sub`, `mul`, `copy` (just `Copy`/`Clone`),
    `swap` (slice `swap`), `set_one()`, `unit_root_recip(n: usize) -> FftComplex`
    computing `(cos(2π/n), -sin(2π/n))` in f32.
  - Derive `Copy, Clone, Debug`.

### Function mapping
- `pub fn fft(x: &[FftComplex], X: &mut [FftComplex], logsize: usize)`
  — out-of-place; `x.len()` must equal `1 << logsize` (add a `debug_assert!`
  or `assert!`).
- `pub fn fft_inplace(x: &mut [FftComplex], logsize: usize)`
  — in-place.
- Private helpers: `fft_clz` → `usize::leading_zeros`;
  `next_reversed_n(reversed_n: usize, shift: usize) -> usize`;
  `rader`, `rader_inplace`, `fft_raw`, and a `do_butterfly(x: &mut [FftComplex], step: usize)`
  function replacing the `DO_BUTTERFLY` macro.

### Key translation notes / risks
1. **f32 vs f64**: must use `f32` throughout, including `PI`
   (`std::f32::consts::PI`) and `cos`/`sin` on `f32`, to preserve the exact
   numeric behavior the C test asserts on. Using f64 would change rounding
   and could break the exact-equality assertions.
2. **`logsize == 0` in-place path**: C `rader_inplace` computes
   `size - 1` with `size = 1`, which is fine (loop `n=1; n<0` doesn't run
   since `size-1 == 0`... actually `n < size - 1` → `1 < 0` false, OK).
   For `logsize == 0`, `size = 1`, loop doesn't execute — safe. Keep the
   same structure; no underflow in Rust since `size >= 1` for `logsize >= 0`.
3. **`next_reversed_n` bit ops**: `~reversed_n` and shifts on `usize` map
   1:1. `INTBITS(size_t)` → `usize::BITS`. The `| 1 << (BITS-1)` and shifts
   are all well-defined for `usize`.
4. **`restrict` pointers**: C `restrict` (non-aliasing) becomes Rust
   `&` / `&mut` slices; the out-of-place `fft` takes `x: &[FftComplex]`
   and `X: &mut [FftComplex]` which enforces disjointness.
5. **`likely`/`unlikely`**: drop them (Rust has no stable equivalent);
   the `if logsize == 0/1/2` early returns stay as plain `if`s.
6. **Test**: port `tests/test.c` to `tests/test.rs`:
   ```rust
   use fft::{FftComplex, fft_inplace};
   #[test]
   fn test_inplace() {
       let mut data = [FftComplex{real:1.0,imag:0.0}, ... 8 elements ...];
       fft_inplace(&mut data, 3);
       let output = [ ... {real:8.0,imag:0.0} at index 4 ... ];
       for i in 0..8 { assert_eq!(data[i].real, output[i].real); }
   }
   ```
   Keep exact `assert_eq!` on `real` to mirror the C `assert`.
7. **No `main` needed**: the C test's `main` becomes a `#[test]` function;
   `cargo test` is the test command.

### Verification
- `cargo test` must pass (the single in-place FFT test).
- Optionally add a small extra test for out-of-place `fft` and a few sizes
  (1, 2, 4, 8) to guard the butterfly stages, but the required test is the
  ported one.
