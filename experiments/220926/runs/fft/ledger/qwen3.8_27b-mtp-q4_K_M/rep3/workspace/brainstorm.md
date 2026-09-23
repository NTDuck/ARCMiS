# Brainstorm: Translating a Small C FFT Library to Rust (cargo test)

This document lists the core difficulties of porting a small C FFT library to
Rust and candidate approaches for each. No code is written here — it is a
planning document only.

## 1. Bit-reversal via `__builtin_clz`

**Difficulty.** The C code likely uses `__builtin_clz` (count leading zeros) to
compute bit-reversed indices. `__builtin_clz` is a GCC/Clang builtin with
undefined behavior on zero, and it is not portable to stable Rust directly.

**Candidate approaches.**
- Use `usize::leading_zeros` (stable since 1.0) — direct semantic equivalent.
- Guard against the zero input explicitly (C builtin is UB on 0; Rust's
  `leading_zeros(0)` is well-defined and returns the bit width, so behavior
  actually differs — verify the C code never passes 0).
- Consider a lookup-table or iterative bit-reversal to avoid the builtin
  entirely and make intent clearer.
- Decide whether to keep the "reverse all N bits" semantics or "reverse within
  log2(N) bits" — the C code's use of `clz` implicitly depends on the width of
  the type; in Rust we must be explicit about the bit width.

## 2. Macro-based complex-number operations

**Difficulty.** C FFT code often uses macros like `CMPLX`, `crealf`,
`cimagf`, or hand-rolled `#define` pairs for add/sub/mul on float pairs.
Rust has no macros of that form, and the C code may rely on macro
side-effect-free expansion in tight loops.

**Candidate approaches.**
- Use `std::f32::from` / a small `Complex` struct with `#[inline]` methods
  (`add`, `sub`, `mul`, `conj`).
- Use the `num-complex` crate (`num_complex::Complex<f32>`) — but note it
  adds a dependency and may not be as tight as hand-rolled inline ops.
- Use `#[inline(always)]` free functions operating on `(f32, f32)` tuples if
  we want zero allocation and no struct overhead.
- If the C code uses `#define` for twiddle factors or unrolled butterflies,
  consider Rust declarative macros (`macro_rules!`) or `const` generics to
  reproduce the unrolling, or just rely on the optimizer.
- Decide: struct with methods vs. tuple + free functions vs. external crate.
  Structs give named fields (`.re`, `.im`) which read closest to C's
  `creal`/`cimag`.

## 3. `restrict` pointers

**Difficulty.** C FFT code may use `restrict` (or `__restrict`) to tell the
compiler that input and output buffers do not alias, enabling vectorization.
Rust's ownership/borrow system expresses this differently.

**Candidate approaches.**
- In Rust, non-overlapping borrows (`&[f32]` input, `&mut [f32]` output)
  already guarantee no aliasing — the compiler can assume disjointness.
- For in-place FFTs, the C code may use `restrict` on a single buffer; in Rust
  we'd use `&mut [f32]` and index-based access, or split borrows carefully.
- No direct `restrict` keyword needed; the type system covers it.
- Watch out: if the C code passes the same pointer as both in and out with
  `restrict`, that is actually UB in C; the Rust port should make the
  in-place vs. out-of-place distinction explicit in the API.

## 4. In-place vs. out-of-place FFT

**Difficulty.** The C library may have both `fft_inplace(buf, n)` and
`fft(src, dst, n)` variants, or only one. Rust's borrow checker makes
in-place mutation of a slice straightforward (`&mut [f32]`), but out-of-place
with separate input/output is also clean. The difficulty is API design and
ensuring the bit-reversal + butterfly stages work correctly on a single
mutable buffer without temporary copies.

**Candidate approaches.**
- Provide both: `fn fft_inplace(buf: &mut [f32], n: usize)` and
  `fn fft(src: &[f32], dst: &mut [f32], n: usize)`.
- Implement the core as in-place, and have out-of-place call it on a copy
  (or copy-then-transform).
- Use `std::mem::swap` for the butterfly exchanges (no allocation).
- Ensure the slice length is a power of two; decide whether to assert,
  return `Result`, or panic.

## 5. `size_t` vs. `usize`

**Difficulty.** C uses `size_t` (unsigned, platform-width) for lengths and
indices. Rust uses `usize`. The mapping is 1:1, but there are subtleties:
- C `size_t` arithmetic wraps silently; Rust debug builds panic on overflow,
  release builds wrap (with `wrapping_*` or default release semantics).
- C code may do `n >> 1` where `n` is `size_t`; in Rust, `usize >> 1` is fine.
- C `sizeof` / array indexing with `size_t` maps to Rust slice indexing with
  `usize`.

**Candidate approaches.**
- Use `usize` everywhere for lengths and indices.
- Use `wrapping_mul` / `wrapping_add` if the C code relies on wraparound
  (unlikely for FFT sizes, but check).
- Use `checked_*` variants or `debug_assert!` for size validation.
- For the bit-reversal computation, ensure shifts are on `usize` and the
  bit-width is `usize::BITS` or `n.trailing_zeros()` as appropriate.

## 6. `M_PI`

**Difficulty.** `M_PI` is a C math-library constant (from `<math.h>`,
technically not in the C standard but universally available). Rust's
`std::f32::consts::PI` and `std::f64::consts::PI` are the equivalents.
The C code may compute twiddle factors as `cos(2.0 * M_PI * k / n)` and
`sin(...)`.

**Candidate approaches.**
- Use `std::f32::consts::PI` (or `f64` if the C code uses double).
- Precompute twiddle factors in a `Vec` or array at init time, or compute
  on the fly per stage.
- If the C code hardcodes `3.14159265...`, replace with the constant for
  precision.
- Consider `f32::cos` / `f32::sin` vs. `f64` — match the C code's precision.

## 7. Test harness

**Difficulty.** C tests may use a custom `main` with `assert`, a framework
like CUnit/Check, or just `printf`-based checks. Rust uses `cargo test` with
the built-in test harness (`#[test]` functions, `assert!`, `assert_eq!`,
`approx` for floating-point).

**Candidate approaches.**
- Port each C test case to a `#[test]` function in `tests/` or inline
  `#[cfg(test)] mod tests`.
- Use `assert!((result - expected).abs() < 1e-5)` for float comparisons, or
  the `float-cmp` / `approx` crates.
- Test vectors: known FFT outputs for small N (N=2, 4, 8) — compare against
  hand-computed or DFT-referenced values.
- Round-trip test: FFT then IFFT should recover the original signal.
- Edge cases: N=1, N=2, non-power-of-two (if supported), empty input.
- Property-based testing with `proptest` if desired (optional).
- Ensure `cargo test` runs all tests; no separate test binary needed.

## Summary of key decisions to make

| Area | Decision |
|------|----------|
| Complex type | Struct vs. tuple vs. `num-complex` crate |
| API shape | In-place only, out-of-place only, or both |
| Precision | `f32` (match C `float`) or `f64` (match C `double`) |
| Size validation | Panic, `Result`, or `debug_assert!` |
| Twiddle factors | Precomputed table vs. on-the-fly `sin`/`cos` |
| Dependencies | Zero-dep (std only) vs. `num-complex` vs. `float-cmp` |
| Test style | Inline `#[cfg(test)]` vs. integration tests in `tests/` |
