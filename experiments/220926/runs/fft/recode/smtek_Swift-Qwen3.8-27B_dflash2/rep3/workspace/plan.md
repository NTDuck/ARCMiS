# Implementation Plan: C FFT library → Rust

## Part A — source files (bottom-up dependency order)

1. `Cargo.toml`
   - Package `fft`, edition 2021, no dependencies. Already written.

2. `src/lib.rs`
   - The entire library in one file (mirrors `src/fft.h` + `src/fft.c`).
   - Fill in, in this order:
     1. `Complex` inherent methods: `add`, `sub`, `mul`, `self_mul`,
        `set_one`, `unit_root_recip` (use `f32::cos`/`f32::sin` and
        `std::f32::consts::PI`; keep f32, not f64).
     2. `fft_clz(n)` → `n.leading_zeros() as usize`.
     3. `next_reversed_n(reversed_n, shift)` → port the C bit-manipulation
        verbatim with `usize` and `usize::BITS` in place of `INTBITS(size_t)`.
     4. `rader(array, target, logsize)` → out-of-place bit-reversal copy.
     5. `rader_inplace(array, logsize)` → in-place bit-reversal swap loop
        (starts at `n = 1`, `reversed_n = size >> 1`, stops at `size - 1`).
     6. `butterfly(x, step)` → port the `DO_BUTTERFLY` macro: twiddle
        `unit = Complex::unit_root_recip(step)`, specialized `i==0`/`i==1`
        cases, generic loop with `self_mul` for the rest.
     7. `fft_raw(x, logsize)` → early returns for `logsize == 0` and `1`,
        `butterfly(x, 2)`, `butterfly(x, 4)`, then loop `step = 8; step <=
        1 << logsize; step *= 2`.
     8. `pub fn fft(x, out, logsize)` → `rader(x, out, logsize); fft_raw(out, logsize)`.
     9. `pub fn fft_inplace(x, logsize)` → `rader_inplace(x, logsize); fft_raw(x, logsize)`.
   - Drop `likely`/`unlikely` hints (plain `if`).
   - Keep all arithmetic in `f32` for bit-identical twiddle precision.

## Part B — test files (bottom-up dependency order)

1. `tests/test.rs`
   - Depends on `src/lib.rs` (Part A item 2).
   - `test_8_point_inplace`: mirrors C `tests/test.c` — 8-point in-place FFT
     of alternating 1/-1, assert exact real parts `{0,0,0,0,8,0,0,0}`.
   - `test_8_point_out_of_place`: same data through `fft`, same assertions
     (covers the `rader` non-inplace path not exercised by the C test).

## Verification
- `cargo test` must pass.
