# FFT Library Translation: C → Rust Design Document

## Source Project Summary

A lightweight Cooley-Tukey FFT implementation in C with:
- Out-of-place and in-place transform variants
- Bit-reversal permutation (iterative in-place algorithm)
- Step-size optimized butterfly computation (supports steps 2, 4, 8, ...)
- Branch prediction hints via `__builtin_clz` (GCC/Clang)
- Static library (`libfft.a`) with coverage/profile flags for testing

## Target Language: Rust

## Module Structure

```
src/
├── lib.rs          # Public API re-export
├── fft.rs         # Core FFT implementation
├── fft_complex.rs # fft_complex_t type + associated utilities
└── tests/
    └── lib_test.rs  # Unit tests
```

## Dependency Mapping

| C Library | Rust Equivalent |
|---|---|
| `<math.h>` (cos, sin, M_PI) | `std::f64::consts::PI`, `extern "C"` bindings or `num-complex` crate |
| `<assert.h>` | `assert!` macro (built-in) |
| `<stddef.h>` | `std::size_t` (built-in) |

No third-party crates required — the library is self-contained.

## Rust Implementation Design

### fft_complex_t

```rust
#[repr(packed)]  // 8 bytes total: 4 bytes real + 4 bytes imag
#[derive(Clone, Copy)]
pub struct FftComplex {
    pub real: f64,
    pub imag: f64,
}
```

Using `repr(packed)` for 8-byte alignment matching the C struct layout. Alternatively, `#[repr(4)]` with two `f64` fields preserves the C struct size.

### FFT Functions

#### fft_complex_add/sub/mul/selfm/copy/swap/setone

These are straightforward — just delegate to the corresponding Rust math operations:

```rust
macro_rules! fft_complex_add {
    ($($t:ident),* => ($result)) => {
        let $($t: f64) => {
            let r = $t.real + $t.real.imag;
            let i = $t.real.imag + $t.real.imag;
            (r, i)
        };
    };
}
```

Actually, let me write clean implementations:

```rust
macro_rules! fft_complex_add {
    ($($t:ident),*) => {
        let mut result = FftComplex { real: 0.0, imag: 0.0 };
        for $t in &[/* ... */] {
            result.real += $t.real;
            result.imag += $t.imag;
        }
        (result)
    };
}
```

Better approach — use a helper:

```rust
macro_rules! fft_complex_op {
    ($($t:ident),*) => {
        let mut result = FftComplex { real: 0.0, imag: 0.0 };
        for $t in &[/* ... */] {
            result.real += $t.real;
            result.imag += $t.imag;
        }
        (result)
    };
}
```

Let me simplify. The macros in C are simple inline functions. In Rust, I'll use a trait-based approach for extensibility.

### fft_raw (Core Algorithm)

The C code has a special structure:
1. If logsize == 0: return early
2. DO_BUTTERFLY(step=2)
3. If logsize == 1: return (size=2)
4. DO_BUTTERFLY(step=4)
5. If logsize == 2: return (size=4)
6. Loop: step=8,16,32,... up to 2^logsize

Each DO_BUTTERFLY does:
- For step=2: butterfly pairs (0,1) with twiddle factor unit root
- For step=4: butterfly pairs (0,1), (2,3) with same twiddle
- For step=8: butterfly pairs (0,1), (2,3), (4,5), (6,7) with twiddle unit root
- For step=N: butterfly pairs (i, i+half) where h = N/2

The key insight: for step=N, the twiddle factor is `unit_root^((i-half)*N/2)` which simplifies to `unit_root^(i-half)`.

In Rust, I'll implement this as:

```rust
fn fft_raw(x: &mut [FftComplex], logsize: usize) {
    if logsize == 0 {
        return;
    }
    let size = 1 << logsize;
    let half = size >> 1;
    
    // Step 2
    do_butterfly_step(x, 2, half);
    
    if logsize == 1 {
        return;
    }
    
    // Step 4
    do_butterfly_step(x, 4, half);
    
    if logsize == 2 {
        return;
    }
    
    // Generic loop
    let mut step = 8;
    while step <= size {
        do_butterfly_step(x, step, half);
        step *= 2;
    }
}
```

### Bit-Reversal Permutation

The C code has two variants:
- `rader`: out-of-place, copies input to reversed order into target
- `rader_inplace`: in-place bit-reversal permutation

Both use the same `next_reversed_n` logic. In Rust, I'll implement both.

### Branch Prediction Hints

The C code uses `__builtin_clz` for branch prediction via `likely`/`unlikely`. In Rust, there's no equivalent compiler intrinsics. However, I can use `unsafe` blocks with `extern "C"` to call the same algorithm, or implement the fallback `fft_clz` function directly in Rust.

Since the fallback path is used on non-GCC/Clang, and the optimization path is the common case, I should:
1. Implement `fft_clz` in Rust (the fallback)
2. Use it everywhere — no need for conditional compilation based on compiler

This is actually cleaner than the C approach.

### Test

The test creates data `[1,0], [-1,0], [1,0], ...` (alternating ±1 on real axis) and expects output where first 3 elements are unchanged and rest become `{8.0, 0.0}`. This is a known property of FFT on certain inputs.

## Build System

Rust doesn't use Makefiles. Use `Cargo.toml`:

```toml
[lib]
name = "fft"
crate-type = ["cdylib"]

[profile.release]
opt-level = 3
```

For testing, use `#[cfg(test)]` attributes in the library crate.

## Risks & Considerations

1. **ABI compatibility**: The C library uses `-fprofile-arcs -ftest-coverage` for profiling. In Rust, we can use `profiling` crate but it's optional. The `libfft.a` static library approach won't work directly — Rust uses dynamic linking or embedded crates instead.

2. **Zero-copy vs heap allocation**: C uses stack allocation for `fft_complex_t` (8 bytes each). Rust's `FftComplex` is also 16 bytes on the stack (two f64s). This is fine.

3. **Thread safety**: The FFT is inherently thread-unsafe (mutates input in-place). Document this clearly.

4. **Precision**: C uses `float` (4 bytes). Rust should use `f64` (8 bytes) for better numerical accuracy. This is a deliberate choice — the C test expects specific float values.

5. **No coverage profiling**: The Makefile's `-fprofile-arcs -ftest-coverage` flags are for profiling. In Rust, we can use `proptest` or `criterion` for benchmarking instead.

6. **External math functions**: C uses `cos`, `sin`, `M_PI` from `<math.h>`. In Rust, we can either:
   - Use `extern "C"` bindings to libm
   - Use `num-complex` crate for complex math
   - Use `std::f64::consts::PI` and manual sin/cos

I'll go with `extern "C"` bindings to libm for simplicity and to avoid introducing new dependencies.

## Implementation Plan

1. Create `Cargo.toml` with cdylib crate type
2. Create `src/fft_complex.rs` — type definition and basic operations
3. Create `src/fft.rs` — core FFT implementation
4. Create `src/lib.rs` — re-export
5. Create `tests/` — unit tests
6. Update `Makefile` → remove (not needed in Rust)
7. Run `cargo test`

## API Compatibility

| C Function | Rust Equivalent |
|---|---|
| `fft_complex_t` | `FftComplex` |
| `fft()` | `fft()` |
| `fft_inplace()` | `fft_inplace()` |
| `FFT_COMPLEX_ADD` | `fft_complex_add` |
| `FFT_COMPLEX_SUB` | `fft_complex_sub` |
| `FFT_COMPLEX_MUL` | `fft_complex_mul` |
| `FFT_COMPLEX_SELFMUL` | `fft_complex_selfmul` |
| `FFT_COMPLEX_COPY` | `fft_complex_copy` |
| `FFT_COMPLEX_SWAP` | `fft_complex_swap` |
| `FFT_COMPLEX_SETONE` | `fft_complex_setone` |
| `FFT_COMPLEX_UNITROOT_RECIP` | `fft_unitroot_reciprocal` |
| `fft()` | `fft()` |
| `fft_inplace()` | `fft_inplace()` |
