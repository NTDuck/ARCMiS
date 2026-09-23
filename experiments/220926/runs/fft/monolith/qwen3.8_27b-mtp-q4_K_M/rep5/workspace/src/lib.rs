//! Rust port of the C FFT library.
//!
//! Provides an in-place and out-of-place radix-2 FFT over `f32` complex
//! numbers, mirroring the original C implementation.

use std::f32::consts::PI;

/// Complex number with `f32` components (mirrors `struct fft_complex`).
#[derive(Debug, Clone, Copy, PartialEq)]
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

    /// Reciprocal of the unit root: e^(-i * 2 * pi / n).
    #[inline]
    pub fn unitroot_recip(n: u32) -> Complex {
        let angle = 2.0 * PI / n as f32;
        Complex {
            real: angle.cos(),
            imag: -angle.sin(),
        }
    }
}

/// Compute the next reversed index (mirrors `next_reversed_n`).
#[inline]
fn next_reversed_n(reversed_n: usize, shift: u32) -> usize {
    let mut r = reversed_n << shift;
    if r == 0 {
        return 0;
    }
    let count_leading_ones = usize::BITS - (!r).leading_zeros();
    r <<= count_leading_ones; // remove leading ones
    r |= 1 << (usize::BITS - 1);
    r >> (shift + count_leading_ones)
}

/// Bit-reverse copy (mirrors `rader`).
fn rader(array: &[Complex], target: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    // how many bits should be shifted to move the number to the most significant bit
    let shift = usize::BITS - logsize;
    let mut reversed_n = 0usize;
    for n in 0..size {
        target[reversed_n] = array[n];
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// Bit-reverse permutation in place (mirrors `rader_inplace`).
fn rader_inplace(array: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    let shift = usize::BITS - logsize;
    // nothing should be done for 0 and size - 1.
    let mut reversed_n = size >> 1;
    for n in 1..size - 1 {
        if n < reversed_n {
            array.swap(n, reversed_n);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// One pass of butterflies over the whole array (mirrors `DO_BUTTERFLY`).
fn do_butterfly(x: &mut [Complex], step: usize) {
    let unit = Complex::unitroot_recip(step as u32);
    let half = step / 2;
    for p in x.chunks_mut(step) {
        // i == 0, j == half
        let t = p[half];
        let u = p[0];
        p[0] = u.add(t);
        p[half] = u.sub(t);
        if half <= 1 {
            continue;
        }
        // i == 1, j == half + 1
        let mut root = unit;
        let t = root.mul(p[half + 1]);
        let u = p[1];
        p[1] = u.add(t);
        p[half + 1] = u.sub(t);
        for (i, j) in (2..half).zip((half + 2)..) {
            root = root.mul(unit);
            let t = root.mul(p[j]);
            let u = p[i];
            p[i] = u.add(t);
            p[j] = u.sub(t);
        }
    }
}

/// Raw FFT after bit reversal (mirrors `fft_raw`).
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

/// Out-of-place FFT (mirrors `fft`).
///
/// `logsize` is the base-2 logarithm of the transform size.
pub fn fft(x: &[Complex], out: &mut [Complex], logsize: u32) {
    assert_eq!(x.len(), 1usize << logsize);
    assert_eq!(out.len(), 1usize << logsize);
    rader(x, out, logsize);
    fft_raw(out, logsize);
}

/// In-place FFT (mirrors `fft_inplace`).
///
/// `logsize` is the base-2 logarithm of the transform size.
pub fn fft_inplace(x: &mut [Complex], logsize: u32) {
    assert_eq!(x.len(), 1usize << logsize);
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
