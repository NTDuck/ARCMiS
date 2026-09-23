//! FFT implementation: bit-reversal (Rader) + iterative butterflies.
//!
//! Replaces `src/fft.c`.

use crate::complex::FftComplex;

/// `next_reversed_n`: advance the bit-reversed index.
fn next_reversed_n(reversed_n: usize, shift: usize) -> usize {
    let clz = (!reversed_n).leading_zeros();
    let next = (reversed_n & ((1 << clz) - 1)) | (1 << (clz - 1));
    (next << shift) | (reversed_n >> clz)
}

/// `rader`: copy input into bit-reversed order (out-of-place).
fn rader(array: &[FftComplex], target: &mut [FftComplex], logsize: usize) {
    let shift = usize::BITS - logsize;
    let mut reversed_n = 0;
    for i in 0..array.len() {
        target[i] = array[next_reversed_n(reversed_n, shift)];
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// `rader_inplace`: swap pairs into bit-reversed order in place.
fn rader_inplace(array: &mut [FftComplex], logsize: usize) {
    let shift = usize::BITS - logsize;
    let mut reversed_n = 0;
    for i in 0..array.len() {
        let j = next_reversed_n(reversed_n, shift);
        if i < j {
            array.swap(i, j);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// `DO_BUTTERFLY` macro as a function: one radix-2 butterfly pass.
fn butterfly(x: &mut [FftComplex], step: usize) {
    let half = step / 2;
    let root = FftComplex::unit_root_recip(step);
    let mut i = 0;
    while i < x.len() {
        let mut w = FftComplex::set_one();
        for k in 0..half {
            let a = x[i + k];
            let b = FftComplex::mul(x[i + k + half], w);
            x[i + k] = FftComplex::add(a, b);
            x[i + k + half] = FftComplex::sub(a, b);
            w.self_mul(root);
        }
        i += step;
    }
}

/// `fft_raw`: run butterflies for step = 2, 4, 8, ..., size.
fn fft_raw(x: &mut [FftComplex], logsize: usize) {
    if logsize == 0 {
        return;
    }
    butterfly(x, 2);
    if logsize == 1 {
        return;
    }
    butterfly(x, 4);
    if logsize == 2 {
        return;
    }
    let size = 1 << logsize;
    let mut step = 8;
    while step <= size {
        butterfly(x, step);
        step *= 2;
    }
}

/// Out-of-place forward FFT.
///
/// `x.len()` and `X.len()` must equal `1 << logsize`.
pub fn fft(x: &[FftComplex], X: &mut [FftComplex], logsize: usize) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}

/// In-place forward FFT.
///
/// `x.len()` must equal `1 << logsize`.
pub fn fft_inplace(x: &mut [FftComplex], logsize: usize) {
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
