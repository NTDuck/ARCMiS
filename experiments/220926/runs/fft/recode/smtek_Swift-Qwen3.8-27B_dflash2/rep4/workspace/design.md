# Translation Design: C FFT library → Rust

## 1. Source project analysis

The source is a small, self-contained C library implementing a radix-2
iterative FFT (Cooley–Tukey) with bit-reversal ("rader") permutation.

### Files
- `Makefile` — builds `libfft.a` from `src/fft.c`; `make test` compiles
  `tests/test.c` against the library and runs `test_1`.
- `src/fft.h` — public API:
  - `struct fft_complex { float real; float imag; }` (typedef `fft_complex_t`)
  - `void fft(const fft_complex_t *restrict x, fft_complex_t *restrict X, size_t logsize);`
  - `void fft_inplace(fft_complex_t *x, size_t logsize);`
  - A set of `FFT_COMPLEX_*` macros implementing complex arithmetic
    (ADD, SUB, MUL, SELFMUL, COPY, SWAP, SETONE, UNITROOT_RECIP).
    `UNITROOT_RECIP` computes `e^(-i·2π/N)` via `cos`/`sin` from `<math.h>`.
- `src/fft.c` — implementation:
  - `fft_clz` — leading-zero count via `__builtin_clz*` (GCC/Clang) with a
    portable fallback.
  - `next_reversed_n(reversed_n, shift)` — O(1) bit-reversal successor.
  - `rader` / `rader_inplace` — bit-reversal permutation (out-of-place / in-place).
  - `DO_BUTTERFLY(begin, end, step)` macro — one stage of butterflies.
  - `fft_raw` — runs butterfly stages for step = 2, 4, 8, …, size.
  - Public `fft` = `rader` + `fft_raw`; `fft_inplace` = `rader_inplace` + `fft_raw`.
- `tests/test.c` — single test: 8-element alternating ±1 input,
  `fft_inplace(data, 3)`, asserts `data[i].real == output[i].real` for all i
  (expected: 8.0 at index 4, 0.0 elsewhere).

### Key semantics / gotchas
- `logsize` is the log2 of the transform size; size must be a power of two.
  `logsize == 0` (size 1) is a no-op.
- `size_t` is 64-bit on the build host; the bit-reversal arithmetic
  (`INTBITS(size_t)`, shifts, `| 1 << (INTBITS-1)`) depends on the width of
  `size_t`. In Rust we use `usize` (also 64-bit on the target) and
  `usize::BITS` / `usize::leading_zeros` to mirror the semantics exactly.
- `fft_clz(~reversed_n)` counts leading zeros of the *inverted* value, i.e.
  the run of leading ones of `reversed_n`. In Rust: `(!reversed_n).leading_zeros()`.
- Complex arithmetic is `f32` (float). Keep `f32` in Rust to preserve
  bit-identical results (the test uses exact `==` on `f32` values).
- `likely`/`unlikely` are branch hints; in Rust these are just plain `if`s
  (the optimizer handles it).
- The C code uses `restrict` pointers; Rust's borrow checker enforces this
  naturally (separate `&[Complex]` input and `&mut [Complex]` output).
- No third-party dependencies: only libc/libm (`cos`, `sin`, `M_PI`).

## 2. Dependency mapping

| C dependency | Rust counterpart | Notes |
|---|---|---|
| `<math.h>` (`cos`, `sin`, `M_PI`) | Rust std `f32::cos`, `f32::sin`, `std::f32::consts::PI` | Built-in, no crate needed. |
| `<limits.h>` (`CHAR_BIT`) | `usize::BITS` | Built-in. |
| `__builtin_clz*` | `usize::leading_zeros()` | Built-in. |
| `assert.h` | `assert!` macro / `#[test]` | Built-in. |
| `stdio.h` (`printf`) | `println!` | Built-in. |

**No external crates are required.** The Rust project is a single crate
with zero dependencies, using only the standard library.

## 3. Target project design (Rust)

### Layout
```
Cargo.toml
src/
  lib.rs        # public API: Complex type, fft, fft_inplace
  complex.rs    # (optional) Complex<f32> struct + arithmetic
  fft.rs        # (optional) implementation: rader, butterfly, fft_raw
tests/
  test.rs       # port of tests/test.c
```
A single `src/lib.rs` is also acceptable given the small size; splitting
into `complex.rs` + `fft.rs` mirrors the C header/source split.

### Public API (idiomatic Rust)
```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Complex { pub re: f32, pub im: f32 }

pub fn fft(x: &[Complex], out: &mut [Complex], logsize: usize);
pub fn fft_inplace(x: &mut [Complex], logsize: usize);
```
- `Complex` replaces `fft_complex_t` (fields `real`/`imag` → `re`/`im`,
  or keep `real`/`imag` for 1:1 clarity — either is fine).
- `fft` takes `&[Complex]` input and `&mut [Complex]` output, mirroring the
  `restrict` pair. Document/`debug_assert` that `out.len() == 1 << logsize`.
- `fft_inplace` takes `&mut [Complex]`.
- Complex arithmetic macros become inherent methods on `Complex`
  (`add`, `sub`, `mul`, `copy` = `Copy` derive, `swap` = `mem::swap`,
  `unit_root_recip(n) -> Complex`).

### Implementation notes
- `fft_clz(n)` → `n.leading_zeros()` (usize).
- `next_reversed_n`:
  ```rust
  fn next_reversed_n(mut r: usize, shift: usize) -> usize {
      r <<= shift;
      let c = (!r).leading_zeros();
      r <<= c;
      r |= 1 << (usize::BITS - 1);
      r >> (shift + c)
  }
  ```
  (Watch operator precedence: `r >> (shift + c)`.)
- `rader` / `rader_inplace`: direct translation of the loops;
  `shift = usize::BITS - logsize`.
- `DO_BUTTERFLY` macro → a private function
  `fn butterfly(x: &mut [Complex], step: usize)` operating on slices,
  with `half = step / 2` and the same i/j butterfly pattern.
- `fft_raw`: same staged structure (step 2, 4, then loop 8..=size).
- `unit_root_recip(n)`: `Complex { re: (2.0 * PI / n as f32).cos(), im: -(2.0 * PI / n as f32).sin() }`.
- Keep everything `f32` to preserve exact test equality.

### Tests
`tests/test.rs` (integration test, run by `cargo test`):
```rust
use fft_lib::Complex;
use fft_lib::fft_inplace;

#[test]
fn test_inplace() {
    let mut data = [
        Complex { re: 1.0, im: 0.0 },
        Complex { re: -1.0, im: 0.0 },
        /* ... 8 alternating entries ... */
    ];
    let expected = [0.0, 0.0, 0.0, 0.0, 8.0, 0.0, 0.0, 0.0];
    fft_inplace(&mut data, 3);
    for i in 0..8 {
        assert_eq!(data[i].re, expected[i]);
    }
}
```
Optionally add unit tests in `src/lib.rs` for `fft` (out-of-place) and
edge cases (logsize 0, 1, 2).

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
No `[dependencies]`.

## 4. Risks
1. **Exact float equality in the test**: the C test asserts `==` on `f32`.
   Rust `f32` arithmetic with the same operation order yields identical
   results, so this is safe as long as we keep `f32` and the same
   expression order (e.g., `a.re*b.re - a.im*b.im`). Do not "improve"
   precision to `f64` without updating the test.
2. **`size_t` width assumptions**: the bit-reversal code assumes 64-bit
   `size_t`. Rust `usize` is 64-bit on the test platform; using
   `usize::BITS` keeps it portable and equivalent.
3. **Operator precedence in `next_reversed_n`**: the final right shift
   must be parenthesized as `r >> (shift + c)`.
4. **Macro-to-function translation**: `DO_BUTTERFLY` uses pointer
   arithmetic (`p != end`, `p += step`); translate to slice indexing
   (`for base in (0..len).step_by(step)`) to avoid off-by-one errors.
5. **`restrict` semantics**: Rust borrows enforce disjoint access; the
   out-of-place `fft` signature with separate `&` and `&mut` slices is
   the natural mapping.
6. **`logsize` validation**: C code does not validate; Rust should
   `debug_assert!(x.len() == 1 << logsize)` to catch misuse without
   changing behavior.
