//! FFT library translated from C.
//!
//! Provides in-place and out-of-place radix-2 FFT over `f32` complex numbers.

use std::f32::consts::PI;

/// Complex number with `f32` components, mirroring `struct fft_complex`.
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

    /// Set to 1 + 0i.
    #[inline]
    pub const fn one() -> Self {
        Complex {
            real: 1.0,
            imag: 0.0,
        }
    }

    /// Reciprocal of the unit root of z^N = 1, i.e. e^(-i * 2 * pi / N).
    #[inline]
    pub fn unitroot_recip(n: u64) -> Self {
        let angle = 2.0 * PI / n as f32;
        Complex {
            real: angle.cos(),
            imag: -angle.sin(),
        }
    }
}

/// Number of bits in `usize`.
const USIZE_BITS: u32 = std::mem::size_of::<usize>() as u32 * 8;

/// Count leading zeros of `n` (mirrors `fft_clz`).
#[inline]
fn fft_clz(n: usize) -> u32 {
    n.leading_zeros()
}

/// Mirrors `next_reversed_n` from the C source.
#[inline]
fn next_reversed_n(reversed_n: usize, shift: u32) -> usize {
    let reversed_n = reversed_n << shift;
    let count_leading_ones = fft_clz(!reversed_n);
    let mut r = reversed_n << count_leading_ones; // remove leading ones
    r |= 1usize << (USIZE_BITS - 1);
    r >> (shift + count_leading_ones)
}

/// Bit-reversal permutation copying `array` into `target` (mirrors `rader`).
fn rader(array: &[Complex], target: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    // how many bits should be shifted to move the number to the most significant bit
    let shift = USIZE_BITS - logsize;
    let mut reversed_n = 0usize;
    for n in 0..size {
        target[reversed_n] = array[n];
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// In-place bit-reversal permutation (mirrors `rader_inplace`).
fn rader_inplace(array: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    let shift = USIZE_BITS - logsize;
    // nothing should be done for 0 and 0b111...11 (size - 1).
    let mut reversed_n = size >> 1;
    for n in 1..size - 1 {
        if n < reversed_n {
            array.swap(n, reversed_n);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// Butterfly pass over `x` with the given `step` (mirrors `DO_BUTTERFLY`).
fn do_butterfly(x: &mut [Complex], step: usize) {
    let unit = Complex::unitroot_recip(step as u64);
    let half = step / 2;
    let len = x.len();
    let mut p = 0usize;
    while p < len {
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

/// Core FFT (mirrors `fft_raw`).
fn fft_raw(x: &mut [Complex], logsize: u32) {
    if logsize == 0 {
        return;
    }

    do_butterfly(x, 2);

    if logsize == 1 {
        return;
    }

    do_butterfly(x, 4);

    if logsize == 2 {
        return;
    }

    // do generic butterfly in a loop
    let mut step = 8usize;
    while step <= 1usize << logsize {
        do_butterfly(x, step);
        step *= 2;
    }
}

/// Compute the FFT of `x` into `X` (both of length `1 << logsize`).
pub fn fft(x: &[Complex], X: &mut [Complex], logsize: u32) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}

/// Compute the FFT of `x` in place (length `1 << logsize`).
pub fn fft_inplace(x: &mut [Complex], logsize: u32) {
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
