//! Rust port of the C FFT library.
//!
//! Provides an in-place and out-of-place radix-2 FFT over `f32` complex
//! numbers, mirroring the original C implementation.

/// A complex number with single-precision real and imaginary parts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub real: f32,
    pub imag: f32,
}

impl Complex {
    /// Creates a new complex number.
    #[inline]
    pub fn new(real: f32, imag: f32) -> Self {
        Complex { real, imag }
    }

    /// The multiplicative identity (1 + 0i).
    #[inline]
    pub fn one() -> Self {
        Complex {
            real: 1.0,
            imag: 0.0,
        }
    }

    /// Returns `self + other`.
    #[inline]
    pub fn add(self, other: Complex) -> Complex {
        Complex {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }

    /// Returns `self - other`.
    #[inline]
    pub fn sub(self, other: Complex) -> Complex {
        Complex {
            real: self.real - other.real,
            imag: self.imag - other.imag,
        }
    }

    /// Returns `self * other`.
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
        let angle = 2.0 * std::f32::consts::PI / n as f32;
        Complex {
            real: angle.cos(),
            imag: -angle.sin(),
        }
    }
}

/// Number of bits in a `usize`.
const USIZE_BITS: u32 = std::mem::size_of::<usize>() as u32 * 8;

/// Counts the leading zeros of `n` (like `__builtin_clz` for `usize`).
#[inline]
fn clz(n: usize) -> u32 {
    usize::leading_zeros(n)
}

/// Computes the next reversed index, mirroring the C `next_reversed_n`.
#[inline]
fn next_reversed_n(mut reversed_n: usize, shift: u32) -> usize {
    reversed_n <<= shift;
    let count_leading_ones = clz(!reversed_n);
    // remove leading ones
    reversed_n <<= count_leading_ones;
    reversed_n |= 1usize << (USIZE_BITS - 1);
    reversed_n >>= shift + count_leading_ones;
    reversed_n
}

/// Bit-reversal permutation copying `array` into `target`.
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

/// In-place bit-reversal permutation.
fn rader_inplace(array: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    let shift = USIZE_BITS - logsize;
    // nothing should be done for 0 and size - 1.
    let mut reversed_n = size >> 1;
    for n in 1..size - 1 {
        if n < reversed_n {
            array.swap(n, reversed_n);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// Performs the butterfly operations for a given step size.
fn do_butterfly(array: &mut [Complex], step: usize) {
    let unit = Complex::unitroot_recip(step as u32);
    let half = step / 2;
    let end = array.len();
    let mut p = 0;
    while p + step <= end {
        // i == 0, j == half
        let t = array[p + half];
        let u = array[p];
        array[p] = u.add(t);
        array[p + half] = u.sub(t);
        if half <= 1 {
            p += step;
            continue;
        }
        // i == 1, j == half + 1
        let mut root = unit;
        let t = root.mul(array[p + half + 1]);
        let u = array[p + 1];
        array[p + 1] = u.add(t);
        array[p + half + 1] = u.sub(t);
        let mut i = 2;
        let mut j = half + 2;
        while i < half {
            root = root.mul(unit);
            let t = root.mul(array[p + j]);
            let u = array[p + i];
            array[p + i] = u.add(t);
            array[p + j] = u.sub(t);
            i += 1;
            j += 1;
        }
        p += step;
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

/// Computes the FFT of `x` into `X`.
///
/// `x` and `X` must both have length `2^logsize`.
#[allow(non_snake_case)]
pub fn fft(x: &[Complex], X: &mut [Complex], logsize: u32) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}

/// Computes the FFT of `x` in place.
///
/// `x` must have length `2^logsize`.
pub fn fft_inplace(x: &mut [Complex], logsize: u32) {
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
