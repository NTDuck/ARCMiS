//! Rust port of the C FFT library.
//!
//! Provides an in-place and out-of-place radix-2 FFT over `f32` complex
//! numbers, mirroring the original C implementation.

use std::f32::consts::PI;

/// Complex number with single-precision components, mirroring
/// `struct fft_complex { float real; float imag; }`.
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

    /// Sets `self` to the reciprocal of the unit root of `z^N = 1`,
    /// i.e. `e^(-i * 2 * pi / N)`.
    #[inline]
    pub fn unitroot_recip(n: u32) -> Complex {
        let angle = 2.0 * PI / n as f32;
        Complex {
            real: angle.cos(),
            imag: -angle.sin(),
        }
    }
}

/// Number of bits in a `usize`.
const USIZE_BITS: u32 = std::mem::size_of::<usize>() as u32 * 8;

/// Count leading zeros of a non-zero `usize`.
#[inline]
fn clz(n: usize) -> u32 {
    debug_assert!(n != 0);
    USIZE_BITS - 1 - n.leading_zeros()
}

/// Compute the next reversed index, mirroring the C
/// `next_reversed_n(reversed_n, shift)` helper.
#[inline]
fn next_reversed_n(mut reversed_n: usize, shift: u32) -> usize {
    reversed_n <<= shift;
    let count_leading_ones = clz(!reversed_n);
    // remove leading ones
    reversed_n <<= count_leading_ones;
    reversed_n |= 1usize << (USIZE_BITS - 1);
    // The C code does `reversed_n >>= (shift + count_leading_ones)`;
    // since the top `shift + count_leading_ones` bits are all ones,
    // this is equivalent to a left rotation by that amount.
    reversed_n = reversed_n.rotate_left(shift + count_leading_ones);
    reversed_n
}

/// Bit-reversal permutation copying `array` into `target`, mirroring
/// the C `rader` function.
fn rader(array: &[Complex], target: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    // how many bits should be shifted to move the number to the most
    // significant bit
    let shift = USIZE_BITS - logsize;
    let mut reversed_n = 0usize;
    for n in 0..size {
        target[reversed_n] = array[n];
        if n + 1 < size {
            reversed_n = next_reversed_n(reversed_n, shift);
        }
    }
}

/// In-place bit-reversal permutation, mirroring the C
/// `rader_inplace` function.
fn rader_inplace(array: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    let shift = USIZE_BITS - logsize;
    // nothing should be done for 0 and size - 1 (0b111...11).
    let mut reversed_n = size >> 1;
    for n in 1..size - 1 {
        if n < reversed_n {
            array.swap(n, reversed_n);
        }
        if n + 1 < size - 1 {
            reversed_n = next_reversed_n(reversed_n, shift) & (size - 1);
        }
    }
}

/// Perform the butterfly operations for one stage of the FFT.
/// Mirrors the C `DO_BUTTERFLY(begin, end, step)` macro.
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

/// Core FFT over a slice whose length is `2^logsize`.
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
    let mut step: usize = 8;
    while step <= 1usize << logsize {
        do_butterfly(x, step);
        step *= 2;
    }
}

/// Compute the FFT of `x` into `X`. Both slices must have length
/// `2^logsize`. Mirrors the C `fft` function.
pub fn fft(x: &[Complex], X: &mut [Complex], logsize: u32) {
    assert_eq!(x.len(), 1usize << logsize);
    assert_eq!(X.len(), 1usize << logsize);
    rader(x, X, logsize);
    fft_raw(X, logsize);
}

/// Compute the FFT of `x` in place. `x` must have length `2^logsize`.
/// Mirrors the C `fft_inplace` function.
pub fn fft_inplace(x: &mut [Complex], logsize: u32) {
    assert_eq!(x.len(), 1usize << logsize);
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
