# Plan: Translating a C FFT Library to Rust

## Goals
- Port the C FFT library (forward/inverse transforms, real/complex variants) to idiomatic Rust.
- Preserve numerical behavior and performance characteristics.
- Provide a safe, ergonomic API with a thin unsafe core where needed.

## Structure
1. **Analyze the C source**
   - Identify public API (function signatures, data layouts, in-place vs out-of-place).
   - Note memory management patterns (malloc/free, caller-owned buffers).
   - Identify global/static state (e.g., precomputed twiddle tables).

2. **Design the Rust crate layout**
   - `src/lib.rs` — public API and re-exports.
   - `src/fft.rs` — core transform algorithms (iterative radix-2, Cooley–Tukey).
   - `src/complex.rs` — complex number type (use `num_complex::Complex<f64>` or a local type).
   - `src/twiddle.rs` — twiddle factor table generation/caching.
   - `src/error.rs` — error types (e.g., non-power-of-two size).
   - `tests/` — ported test cases from the C suite.

3. **Translation rules**
   - `double` → `f64`; complex pairs → `Complex<f64>`.
   - Raw pointer + length → slices (`&[T]` / `&mut [T]`).
   - `malloc/free` → caller-provided buffers or `Vec`; no manual deallocation.
   - Return-code errors → `Result<T, FftError>`.
   - Keep hot loops allocation-free; use `#[inline]` and `#[target_feature]` where appropriate.

4. **Performance**
   - Use `std::arch` intrinsics / auto-vectorization for the inner loops.
   - Benchmark against the C original (criterion) to confirm parity.

5. **Testing**
   - Port existing C unit tests to Rust `tests/`.
   - Add property tests: round-trip (forward then inverse ≈ identity), Parseval's theorem, known DFT vectors.
   - Fuzz edge cases: size 1, 2, large powers of two, NaN/Inf inputs.

6. **Build & CI**
   - `Cargo.toml` with no required external deps (or `num-complex` optional).
   - CI: `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check`.

## Milestones
- [ ] M1: API surface + error types compile.
- [ ] M2: Core iterative FFT correct on small sizes.
- [ ] M3: Real-input/real-output variants.
- [ ] M4: Full test suite green.
- [ ] M5: Benchmarks at parity with C.
