//! FFT implementation: bit-reversal permutation ("rader") and
//! iterative butterfly stages.
//!
//! Replaces `src/fft.c`.

use crate::complex::Complex;

/// `next_reversed_n(reversed_n, shift)` — O(1) bit-reversal successor.
fn next_reversed_n(reversed_n: usize, shift: usize) -> usize {
    todo!()
}

/// `rader(array, target, logsize)` — out-of-place bit-reversal permutation.
fn rader(array: &[Complex], target: &mut [Complex], logsize: usize) {
    todo!()
}

/// `rader_inplace(array, logsize)` — in-place bit-reversal permutation.
fn rader_inplace(array: &mut [Complex], logsize: usize) {
    todo!()
}

/// `DO_BUTTERFLY(begin, end, step)` — one butterfly stage over the slice.
fn butterfly(x: &mut [Complex], step: usize) {
    todo!()
}

/// `fft_raw(x, logsize)` — runs butterfly stages for step = 2, 4, 8, ..., size.
fn fft_raw(x: &mut [Complex], logsize: usize) {
    todo!()
}

/// Public out-of-place FFT (replaces C `fft`).
///
/// `x` is the input, `out` the output; both must have length `1 << logsize`.
pub fn fft(x: &[Complex], out: &mut [Complex], logsize: usize) {
    todo!()
}

/// Public in-place FFT (replaces C `fft_inplace`).
///
/// `x` must have length `1 << logsize`.
pub fn fft_inplace(x: &mut [Complex], logsize: usize) {
    todo!()
}
