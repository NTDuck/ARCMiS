# Brainstorm: Translating a C FFT Library to Rust

## Goals
- Port the C FFT library (radix-2/4, in-place, iterative) to idiomatic Rust.
- Preserve numerical behavior and performance characteristics.
- Provide a safe, ergonomic API while keeping a low-level path for power users.

## Key Considerations
- **Complex number type**: Use `num-complex::Complex<f64>` (or a custom `Complex` struct) instead of `cfloat`/`cdouble`.
- **Memory**: C code likely uses `malloc`/`free` and raw pointers. In Rust, use `Vec<Complex<f64>>` and slices; avoid `unsafe` where possible.
- **Bit-reversal permutation**: Implement as a pure function over indices; can be vectorized.
- **Twiddle factors**: Precompute or compute on the fly; consider `#[inline]` and `f64` trig.
- **Error handling**: C returns error codes; Rust should use `Result` or `Debug`-assert invariants (e.g., power-of-two length).
- **Benchmarks**: Use `criterion` to compare against the C baseline.
- **Tests**: Port existing C test vectors; add property-based tests (round-trip FFT/IFFT, Parseval's theorem).

## API Sketch
```rust
pub struct Fft { /* size, twiddles */ }
impl Fft {
    pub fn new(n: usize) -> Result<Self, FftError>;
    pub fn forward(&self, data: &mut [Complex<f64>]);
    pub fn inverse(&self, data: &mut [Complex<f64>]);
}
```

## Risks
- Performance regression from bounds checks (mitigate with `debug_assert` + release builds).
- Subtle numerical differences vs. C (validate with test vectors).
