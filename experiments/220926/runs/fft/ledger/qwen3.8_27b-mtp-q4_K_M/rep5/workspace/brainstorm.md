# Brainstorm: Translating a C FFT Library to Rust

## Goal
Port a C-based Fast Fourier Transform (FFT) library to idiomatic Rust, preserving numerical behavior while gaining memory safety and better ergonomics.

## Key Considerations
- **Data types**: Map `float`/`double` to `f32`/`f64`; represent complex numbers with a `Complex<T>` struct (or a crate like `num-complex`).
- **Memory layout**: C uses raw pointers and manual allocation; Rust should use `Vec<Complex<T>>` and slices. Watch for in-place vs. out-of-place FFT conventions.
- **Performance**: Use `#[inline]`, avoid unnecessary allocations in the recursive/iterative loops, and consider `rayon` for parallel stages.
- **Numerical accuracy**: Keep the same twiddle-factor computation order to match test expectations; validate against the C reference output.
- **API design**: Provide a clean public API (e.g., `fft::forward(&mut data)`, `fft::inverse(&mut data)`) with normalization handled explicitly.
- **Testing**: Port the C test suite; add property-based tests (round-trip forward/inverse, Parseval's theorem).

## Open Questions
- Should the port support arbitrary radix or stay radix-2?
- Use a dependency (`num-complex`) or define our own `Complex` type?
- Keep the same function signatures for drop-in compatibility, or redesign the API?
