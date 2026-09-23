# Translation Plan: C FFT library → Rust

Source: `src/fft.c`, `src/fft.h`, `tests/test.c` (C, single-file FFT library)
Target: Rust cargo package `fft` (zero external dependencies), test command `cargo test`

Skeleton files already written (compilable, all functions stubbed with `todo!`):
- `Cargo.toml` (package `fft`, empty `[dependencies]`, empty `[workspace]` to detach from parent workspace)
- `src/lib.rs` (public `Complex`, `fft`, `fft_inplace`; private `next_reversed_n`, `rader`, `rader_inplace`, `butterfly`, `fft_raw`, `unitroot_recip`)
- `tests/test.rs` (mirror of `tests/test.c`)

## Part A — source files, bottom-up dependency order

Single source file to translate: `src/fft.c` (+ `src/fft.h` macros) → `src/lib.rs`.
Fill the stubs in this order (each step depends only on earlier steps):

1. `Complex` struct + `unitroot_recip(n) -> Complex`
   - From `src/fft.h`: `struct fft_complex` → `pub struct Complex { pub real: f32, pub imag: f32 }` (derive Clone, Copy, Debug, PartialEq).
   - `FFT_COMPLEX_UNITROOT_RECIP` → `fn unitroot_recip(n: usize) -> Complex { let a = 2.0f64 * std::f64::consts::PI / n as f64; Complex { real: a.cos() as f32, imag: -a.sin() as f32 } }` (f64 math then cast, matching C's double cos/sin).
2. `next_reversed_n(reversed_n: usize, shift: u32) -> usize`
   - From `src/fft.c` `next_reversed_n` + `fft_clz`. Use `(!reversed_n).leading_zeros()` for `count_leading_ones`; `usize::BITS` for `INTBITS(size_t)`.
3. `rader(array: &[Complex], target: &mut [Complex], logsize: u32)`
   - From `src/fft.c` `rader`: loop `n = 0..size`, copy `array[n]` to `target[reversed_n]`, advance with `next_reversed_n`.
4. `rader_inplace(array: &mut [Complex], logsize: u32)`
   - From `src/fft.c` `rader_inplace`: loop `n = 1..size-1` starting `reversed_n = size >> 1`; `std::mem::swap` when `n < reversed_n`.
5. `butterfly(array: &mut [Complex], step: usize)`
   - From `DO_BUTTERFLY` macro: for each block of `step`, do the i=0/i=1 pairs then the inner loop `i = 2..half` with `root` self-multiplication (`FFT_COMPLEX_SELFMUL` → inline expression with temporaries). Unit root via `unitroot_recip(step)`.
6. `fft_raw(x: &mut [Complex], logsize: u32)`
   - From `src/fft.c` `fft_raw`: early return for `logsize == 0`; call `butterfly(x, 2)`; return if `logsize == 1`; `butterfly(x, 4)`; return if `logsize == 2`; then `for (let mut step = 8usize; step <= 1usize << logsize; step *= 2) { butterfly(x, step); }`.
7. `pub fn fft(x: &[Complex], X: &mut [Complex], logsize: u32)`
   - `rader(x, X, logsize); fft_raw(X, logsize);`
8. `pub fn fft_inplace(x: &mut [Complex], logsize: u32)`
   - `rader_inplace(x, logsize); fft_raw(x, logsize);`

Notes:
- Drop `likely`/`unlikely` (`__builtin_expect`) — no Rust equivalent, no semantic effect.
- `restrict` is enforced by Rust borrows (`&[Complex]` + `&mut [Complex]` cannot alias).
- `size_t` → `usize` internally; `logsize` stays `u32` in the public API.
- Complex arithmetic macros (`FFT_COMPLEX_ADD/SUB/MUL/SELFMUL/COPY/SWAP`) become inline expressions or `std::mem::swap`; no separate functions needed.

## Part B — test files, bottom-up dependency order

1. `tests/test.rs` (already written, mirrors `tests/test.c`)
   - `test_inplace_8`: 8-point in-place FFT of `[1,-1,1,-1,1,-1,1,-1]` via `fft_inplace(&mut data, 3)`; assert `data[i].real == output[i].real` for all `i` (expected `8.0` at index 4, `0.0` elsewhere).
   - Optional (parity-plus coverage, not in C test): an out-of-place `fft` test on the same data, comparing against the in-place result; use approximate comparison (`(a - b).abs() < 1e-4`) for any non-exact values.

Verification: `cargo test` must pass (currently fails only because `src/lib.rs` stubs are `todo!`).
