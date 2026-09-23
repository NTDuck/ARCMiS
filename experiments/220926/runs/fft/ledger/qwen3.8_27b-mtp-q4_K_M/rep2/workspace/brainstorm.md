# Brainstorm: Translating a C FFT Library to Rust

## Goals
- Faithful port of the C FFT library (radix-2, radix-4, mixed-radix, real/complex transforms) to idiomatic Rust.
- Preserve numerical behavior: same algorithms, same twiddle-factor ordering, same in-place semantics.
- Provide safe public API; keep unsafe confined to low-level hot loops (if any).
- Keep performance close to the C original (SIMD-friendly inner loops, no allocations in the hot path).

## High-level plan
1. **Inventory the C code**
   - List every public function (fft, ifft, rfft, irfft, plan/destroy, etc.).
   - Note data types: `float`/`double`/`cfloat`/`cdouble` → Rust `f32`/`f64`/`Complex<f32>`/`Complex<f64>`.
   - Identify global state (precomputed twiddle tables, bit-reversal tables) → replace with owned structs.
2. **Design the Rust API**
   - Generic over `f32`/`f64` with a `Num`-like trait bound (or just two concrete impls).
   - `struct FftPlan { size: usize, twiddles: Vec<Complex<T>>, bitrev: Vec<usize> }`
   - `FftPlan::new(n)`, `plan.forward(&mut [Complex<T>])`, `plan.inverse(&mut [Complex<T>])`.
   - In-place transforms take `&mut [Complex<T>]`; return `Result<(), FftError>` for invalid sizes.
3. **Memory & ownership**
   - C code likely uses `malloc`/caller-allocated buffers → Rust: caller passes `&mut Vec`/slice; plan owns its tables.
   - No raw pointers in the public API.
4. **Error handling**
   - C returns error codes or NULL → Rust `Result` + `thiserror`-style error enum.
   - Validate: size > 0, power-of-two (or supported mixed radix), non-empty input.
5. **Numerical details**
   - Twiddle factors: `exp(-2πi k/n)` for forward, conjugate for inverse; match C's sign convention.
   - Inverse normalization: divide by `n` (check whether the C lib does this or leaves it to the caller).
   - Bit-reversal: precompute permutation table in the plan.
   - Watch for `float` vs `double` precision differences in tests.
6. **Performance**
   - Use `#[inline]` on butterfly kernels; consider `std::simd` (portable_simd) later.
   - Avoid bounds checks in inner loops: use `split_at_mut` / `get_unchecked` only if justified.
   - Benchmark against the C build with `criterion`.
7. **Testing**
   - Port every C unit test to Rust `#[test]` (or `cargo test`).
   - Round-trip tests: `ifft(fft(x)) ≈ x` within tolerance.
   - Known-value tests: DFT of small vectors computed by hand.
   - Property tests (`proptest`): random signals, round-trip, Parseval's theorem.
   - Fuzz entry point for the transform kernel.
8. **Crate layout**
   ```
   fft-rs/
     Cargo.toml
     src/
       lib.rs        # re-exports, docs
       plan.rs       # FftPlan, twiddle/bitrev tables
       complex.rs    # small Complex<T> if not using num-complex
       forward.rs    # in-place DIT/DIF kernels
       inverse.rs
       real.rs       # packed real transforms
       error.rs
     tests/
       roundtrip.rs
       known_values.rs
       property.rs
     benches/
       fft_bench.rs
   ```
9. **Dependencies**
   - `num-complex` (or hand-rolled `Complex` to avoid dep), `thiserror`, `criterion` (dev), `proptest` (dev).
10. **Risks / open questions**
    - Does the C lib support arbitrary (non-power-of-two) sizes? If so, port Bluestein or mixed-radix.
    - Threaded/multi-buffer APIs? Decide whether to port or drop.
    - ABI compatibility with existing C callers? (Probably out of scope; pure Rust API.)
    - Licensing: check C source license before vendoring test vectors.

## First concrete steps
- [ ] Copy C sources into `vendor/` for reference (not compiled).
- [ ] Write `Cargo.toml`, skeleton `lib.rs` with `FftPlan` and error type.
- [ ] Port radix-2 forward transform + one known-value test.
- [ ] Port inverse + round-trip test.
- [ ] Port remaining transforms, then benchmarks.
