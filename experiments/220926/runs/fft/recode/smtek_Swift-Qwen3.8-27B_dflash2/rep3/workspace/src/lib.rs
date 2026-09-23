//! Radix-2 DIT FFT library (port of the C `libfft.a`).
//!
//! Public API:
//! - [`Complex`]: 32-bit float complex number (mirrors `fft_complex_t`).
//! - [`fft`]: out-of-place forward FFT of `2^logsize` points.
//! - [`fft_inplace`]: in-place forward FFT of `2^logsize` points.

use std::f32::consts::PI;

/// A 32-bit float complex number, mirroring C `struct fft_complex`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub real: f32,
    pub imag: f32,
}

impl Complex {
    /// `FFT_COMPLEX_ADD`: `result = a + b`.
    pub fn add(a: Complex, b: Complex) -> Complex {
        Complex {
            real: a.real + b.real,
            imag: a.imag + b.imag,
        }
    }

    /// `FFT_COMPLEX_SUB`: `result = a - b`.
    pub fn sub(a: Complex, b: Complex) -> Complex {
        Complex {
            real: a.real - b.real,
            imag: a.imag - b.imag,
        }
    }

    /// `FFT_COMPLEX_MUL`: `result = a * b`.
    pub fn mul(a: Complex, b: Complex) -> Complex {
        Complex {
            real: a.real * b.real - a.imag * b.imag,
            imag: a.real * b.imag + a.imag * b.real,
        }
    }

    /// `FFT_COMPLEX_SELFMUL`: `self = self * z` (in place).
    pub fn self_mul(&mut self, z: Complex) {
        *self = Complex::mul(*self, z);
    }

    /// `FFT_COMPLEX_SETONE`: `result = 1 + 0i`.
    pub fn set_one() -> Complex {
        Complex { real: 1.0, imag: 0.0 }
    }

    /// `FFT_COMPLEX_UNITROOT_RECIP`: `e^(-i * 2 * pi / n)`, computed in f32.
    pub fn unit_root_recip(n: usize) -> Complex {
        let angle = 2.0 * PI / n as f32;
        Complex {
            real: angle.cos(),
            imag: -angle.sin(),
        }
    }
}

/// `fft_clz` in C: count leading zeros of `n` (maps to `usize::leading_zeros`).
fn fft_clz(n: usize) -> usize {
    n.leading_zeros() as usize
}

/// Next bit-reversed index, mirroring C `next_reversed_n`.
fn next_reversed_n(reversed_n: usize, shift: usize) -> usize {
    let clz = fft_clz(!reversed_n);
    let low = (reversed_n >> (clz + 1)) << 1;
    let high = (1 << (clz + 1)) - 1;
    (low | (high << shift))
}

/// Bit-reversal permutation (out of place), mirroring C `rader`.
fn rader(array: &[Complex], target: &mut [Complex], logsize: usize) {
    let size = 1 << logsize;
    let mut n = 0;
    let mut reversed_n = 0;
    loop {
        target[reversed_n] = array[n];
        if n == size - 1 {
            break;
        }
        n += 1;
        reversed_n = next_reversed_n(reversed_n, usize::BITS - logsize as u32);
    }
}

/// Bit-reversal permutation (in place), mirroring C `rader_inplace`.
fn rader_inplace(array: &mut [Complex], logsize: usize) {
    let size = 1 << logsize;
    let mut n = 1;
    let mut reversed_n = size >> 1;
    while n < size - 1 {
        array.swap(n, reversed_n);
        n += 1;
        reversed_n = next_reversed_n(reversed_n, usize::BITS - logsize as u32);
    }
}

/// One radix-2 DIT butterfly pass, mirroring the C `DO_BUTTERFLY` macro.
fn butterfly(x: &mut [Complex], step: usize) {
    let size = x.len();
    let unit = Complex::unit_root_recip(step);
    let half = step / 2;
    let mut i = 0;
    while i < size {
        let mut j = i;
        let mut k = i + half;
        // i == 0: twiddle is 1, no multiplication needed.
        let a = x[j];
        let b = x[k];
        x[j] = Complex::add(a, b);
        x[k] = Complex::sub(a, b);
        // i == 1: twiddle is -i, specialized.
        let a = x[j + 1];
        let b = x[k + 1];
        x[j + 1] = Complex {
            real: a.real + b.imag,
            imag: a.imag - b.real,
        };
        x[k + 1] = Complex {
            real: a.real - b.imag,
            imag: a.imag + b.real,
        };
        // Generic case: twiddle by powers of `unit`.
        let mut twiddle = Complex::mul(unit, unit);
        let mut t = 2;
        while t < half {
            let a = x[j + t];
            let b = Complex::mul(x[k + t], twiddle);
            x[j + t] = Complex::add(a, b);
            x[k + t] = Complex::sub(a, b);
            twiddle.self_mul(unit);
            t += 1;
        }
        i += step;
    }
}

/// Butterfly passes for step = 2, 4, 8, ..., 2^logsize, mirroring C `fft_raw`.
fn fft_raw(x: &mut [Complex], logsize: usize) {
    if logsize == 0 {
        return;
    }
    if logsize == 1 {
        butterfly(x, 2);
        return;
    }
    butterfly(x, 2);
    butterfly(x, 4);
    let mut step = 8;
    while step <= 1 << logsize {
        butterfly(x, step);
        step *= 2;
    }
}

/// Out-of-place forward FFT of `2^logsize` points.
///
/// `out.len()` must be at least `1 << logsize`.
pub fn fft(x: &[Complex], out: &mut [Complex], logsize: usize) {
    rader(x, out, logsize);
    fft_raw(out, logsize);
}

/// In-place forward FFT of `2^logsize` points.
pub fn fft_inplace(x: &mut [Complex], logsize: usize) {
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
