//! FFT library translated from C.
//!
//! Provides in-place and out-of-place radix-2 FFT over `f32` complex numbers.

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
    pub const fn one() -> Self {
        Complex {
            real: 1.0,
            imag: 0.0,
        }
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
        let angle = 2.0 * std::f32::consts::PI / n as f32;
        Complex {
            real: angle.cos(),
            imag: -angle.sin(),
        }
    }
}

/// Number of leading zeros of `n`, mirroring `fft_clz`.
#[inline]
fn fft_clz(n: u64) -> u32 {
    n.leading_zeros()
}

/// Mirrors `next_reversed_n`.
#[inline]
fn next_reversed_n(reversed_n: u64, shift: u32) -> u64 {
    let mut r = reversed_n << shift;
    let count_leading_ones = fft_clz(!r);
    r <<= count_leading_ones; // remove leading ones
    r |= 1u64 << 63;
    r >>= shift + count_leading_ones;
    r
}

/// Bit-reversal permutation into a separate buffer, mirroring `rader`.
fn rader(array: &[Complex], target: &mut [Complex], logsize: u32) {
    let size = 1u64 << logsize;
    // how many bits should be shifted to move the number to the most significant bit
    let shift = 64 - logsize;
    let mut reversed_n = 0u64;
    for n in 0..size {
        target[reversed_n as usize] = array[n as usize];
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// In-place bit-reversal permutation, mirroring `rader_inplace`.
fn rader_inplace(array: &mut [Complex], logsize: u32) {
    let size = 1u64 << logsize;
    let shift = 64 - logsize;
    // nothing should be done for 0 and 0b111...11 (size - 1).
    let mut reversed_n = size >> 1;
    for n in 1..size - 1 {
        if n < reversed_n {
            array.swap(n as usize, reversed_n as usize);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// Butterfly pass over the whole array, mirroring `DO_BUTTERFLY`.
fn do_butterfly(x: &mut [Complex], step: u64) {
    let unit = Complex::unitroot_recip(step);
    let half = (step / 2) as usize;
    let step_us = step as usize;
    let mut p = 0usize;
    while p + step_us <= x.len() {
        // i == 0, j == half
        let t = x[p + half];
        let u = x[p];
        x[p] = u.add(t);
        x[p + half] = u.sub(t);
        if half <= 1 {
            p += step_us;
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
        p += step_us;
    }
}

/// Core FFT without bit-reversal, mirroring `fft_raw`.
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
    let mut step: u64 = 8;
    while step <= 1u64 << logsize {
        do_butterfly(x, step);
        step *= 2;
    }
}

/// Out-of-place FFT: reads `x`, writes the transformed result to `X`.
/// `logsize` is the base-2 logarithm of the transform size.
#[allow(non_snake_case)]
pub fn fft(x: &[Complex], X: &mut [Complex], logsize: u32) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}

/// In-place FFT. `logsize` is the base-2 logarithm of the transform size.
pub fn fft_inplace(x: &mut [Complex], logsize: u32) {
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
