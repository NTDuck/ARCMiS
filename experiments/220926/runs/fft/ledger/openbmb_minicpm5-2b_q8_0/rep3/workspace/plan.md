# FFT Library Translation Plan: C → Rust

## Strategy

This is a **core numerical library translation** with significant challenges:

1. **Numerical precision**: FFT involves floating-point arithmetic; Rust's `f32`/`f64` must match C's `float`/`double` exactly
2. **Bit-reversal permutation**: Requires careful indexing and in-place operations
3. **Butterfly operations**: Complex arithmetic (ADD, SUB, MUL, SWAP) need correct Rust equivalents
4. **Test data alignment**: The test uses specific input/output patterns that must reproduce exactly
5. **Build system**: C's Makefile with profiling flags needs Rust equivalent (Cargo.toml + build.rs or cargo test)

### Key Decisions
- Use `f64` for FFT computations (matches C `double`)
- Implement bit-reversal using `usize` indexing (Rust's natural integer type)
- Use `Vec<[f64; 2]>` or `[f64; 2]` for complex numbers
- Write Rust equivalent of C test data and expected output

### Risks
- Bit-reversal algorithm must produce identical results to C implementation
- Test data: alternating [1, -1] pattern with expected [0,0,0,0,8,0,0,0]
- The test uses `fft_inplace()` with logsize=3 (size=8) — this is a critical detail

### Approach
1. Start with brainstorm task to identify core difficulties
2. Translate core data structures and functions
3. Implement test harness
4. Verify against C test output
5. Build and test

## Tasks
1. Brainstorm: Core difficulties and candidate approaches
2. Translate core data structures (fft_complex, macros, functions)
3. Implement bit-reversal permutation
4. Implement FFT butterfly operations
5. Implement public API (fft, fft_inplace)
6. Write test harness and verify against C test data
7. Final verification
