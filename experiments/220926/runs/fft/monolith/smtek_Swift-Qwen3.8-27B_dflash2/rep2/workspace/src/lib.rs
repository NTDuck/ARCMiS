//! FFT library translated from the C source.
//!
//! Provides an in-place and out-of-place radix-2 FFT over `f32` complex
//! numbers, using a bit-reversal permutation (Rader) followed by
//! iterative butterflies.

use std::f32::consts::PI;

/// Complex number with single-precision components.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Complex {
    pub real: f32,
    pub imag: f32,
}

impl Complex {
    #[inline]
    pub const fn new(real: f32, imag: f32) -> Self {
        Complex { real, imag }
    }

    #[inline]
    pub fn add(self, other: Complex) -> Complex {
        Complex {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }

    #[inline]
    pub fn sub(self, other: Complex) -> Complex {
        Complex {
            real: self.real - other.real,
            imag: self.imag - other.imag,
        }
    }

    #[inline]
    pub fn mul(self, other: Complex) -> Complex {
        Complex {
            real: self.real * other.real - self.imag * other.imag,
            imag: self.real * other.imag + self.imag * other.real,
        }
    }

    /// Reciprocal of the unit root of z^N = 1, i.e. e^(-i * 2 * pi / N).
    #[inline]
    pub fn unitroot_recip(n: u64) -> Complex {
        let angle = 2.0 * PI / n as f32;
        Complex {
            real: angle.cos(),
            imag: -angle.sin(),
        }
    }
}

/// Number of bits in `u64`.
const INTBITS: u32 = 64;

/// Count leading zeros of a `u64` (mirrors `fft_clz`).
#[inline]
fn fft_clz(n: u64) -> u32 {
    n.leading_zeros()
}

/// Mirrors `next_reversed_n` from the C source.
#[inline]
fn next_reversed_n(reversed_n: u64, shift: u32) -> u64 {
    let mut r = reversed_n << shift;
    let count_leading_ones = fft_clz(!r);
    r <<= count_leading_ones; // remove leading ones
    r |= 1u64 << (INTBITS - 1);
    r >>= shift + count_leading_ones;
    r
}

/// Out-of-place bit-reversal permutation (mirrors `rader`).
fn rader(array: &[Complex], target: &mut [Complex], logsize: u32) {
    let size = 1u64 << logsize;
    let shift = INTBITS - logsize;
    for n in 0..size {
        let reversed_n = next_reversed_n(n, shift);
        target[reversed_n as usize] = array[n as usize];
    }
}

/// In-place bit-reversal permutation (mirrors `rader_inplace`).
fn rader_inplace(array: &mut [Complex], logsize: u32) {
    let size = 1u64 << logsize;
    let shift = INTBITS - logsize;
    // nothing should be done for 0 and size - 1.
    for n in 1..size - 1 {
        let reversed_n = next_reversed_n(n, shift);
        if n < reversed_n {
            array.swap(n as usize, reversed_n as usize);
        }
    }
}

/// Iterative butterflies (mirrors `fft_raw`).
fn fft_raw(x: &mut [Complex], logsize: u32) {
    if logsize == 0 {
        return;
    }

    let size = 1usize << logsize;
    let begin = 0;
    let end = size;

    // DO_BUTTERFLY(begin, end, 2)
    butterfly(x, begin, end, 2);

    if logsize == 1 {
        return;
    }

    // DO_BUTTERFLY(begin, end, 4)
    butterfly(x, begin, end, 4);

    if logsize == 2 {
        return;
    }

    // generic butterfly in a loop
    let mut step = 8usize;
    while step <= size {
        butterfly(x, begin, end, step);
        step *= 2;
    }
}

/// Mirrors the `DO_BUTTERFLY` macro.
fn butterfly(x: &mut [Complex], begin: usize, end: usize, step: usize) {
    let unit = Complex::unitroot_recip(step as u64);
    let half = step / 2;
    let mut p = begin;
    while p != end {
        // i == 0, j == half
        let t = x[p + half];
        let u = x[p];
        x[p] = u.add(t);
        x[p + half] = u.sub(t);
        if half <= 1 {
            p += step;
            continue;
        }
        // i == 1, j == half + 1
        let mut root = unit;
        let t = root.mul(x[p + half + 1]);
        let u = x[p + 1];
        x[p + 1] = u.add(t);
        x[p + half + 1] = u.sub(t);
        let mut i = 2usize;
        let mut j = half + 2;
        while i < half {
            root = root.mul(unit);
            let t = root.mul(x[p + j]);
            let u = x[p + i];
            x[p + i] = u.add(t);
            x[p + j] = u.sub(t);
            i += 1;
            j += 1;
        }
        p += step;
    }
}

/// Compute the FFT of `x` into `X`. `logsize` is the base-2 logarithm of the
/// transform size.
pub fn fft(x: &[Complex], X: &mut [Complex], logsize: u32) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}

/// Compute the FFT of `x` in place. `logsize` is the base-2 logarithm of the
/// transform size.
pub fn fft_inplace(x: &mut [Complex], logsize: u32) {
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
