//! Rust port of the C FFT library (src/fft.c + src/fft.h).
//!
//! Public API:
//! - `Complex` — single-precision complex number `{ real: f32, imag: f32 }`
//! - `fft(x, X, logsize)` — out-of-place radix-2 DIT FFT of `2^logsize` points
//! - `fft_inplace(x, logsize)` — in-place radix-2 DIT FFT of `2^logsize` points

/// Single-precision complex number (mirrors C `struct fft_complex`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Complex {
    pub real: f32,
    pub imag: f32,
}

/// Out-of-place FFT: reads `x` (length `1 << logsize`), writes result to `X`.
pub fn fft(x: &[Complex], X: &mut [Complex], logsize: u32) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}

/// In-place FFT: transforms `x` (length `1 << logsize`) in place.
pub fn fft_inplace(x: &mut [Complex], logsize: u32) {
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Bit-reversal successor generator (C: `next_reversed_n`).
///
/// Given the current reversed index `reversed_n` and `shift = BITS - logsize`,
/// returns the next reversed index. Uses `leading_zeros` of `!reversed_n`.
fn next_reversed_n(mut reversed_n: usize, shift: u32) -> usize {
    reversed_n <<= shift;
    let count_leading_ones = (!reversed_n).leading_zeros() as u32;
    reversed_n <<= count_leading_ones;
    reversed_n |= 1 << (usize::BITS - 1);
    reversed_n >>= shift + count_leading_ones;
    reversed_n
}

/// Out-of-place bit-reversal permutation (C: `rader`).
fn rader(array: &[Complex], target: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    let shift = usize::BITS - logsize;
    let mut reversed_n = 0;
    for n in 0..size {
        target[reversed_n] = array[n];
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// In-place bit-reversal permutation (C: `rader_inplace`).
fn rader_inplace(array: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    let shift = usize::BITS - logsize;
    let mut reversed_n = 0;
    for n in 0..size {
        if n != reversed_n {
            array.swap(n, reversed_n);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// One radix-2 DIT butterfly stage over `array` with block size `step`
/// (C: `DO_BUTTERFLY` macro).
fn butterfly(array: &mut [Complex], step: usize) {
    let unit = unitroot_recip(step);
    for chunk in array.chunks_mut(step) {
        let half = step / 2;
        let mut root = Complex { real: 1.0, imag: 0.0 };
        for k in 0..half {
            let a = chunk[k];
            let b = chunk[k + half];
            // FFT_COMPLEX_MUL(b, root)
            let br = Complex {
                real: b.real * root.real - b.imag * root.imag,
                imag: b.real * root.imag + b.imag * root.real,
            };
            // FFT_COMPLEX_ADD(a, br) / FFT_COMPLEX_SUB(a, br)
            chunk[k] = Complex {
                real: a.real + br.real,
                imag: a.imag + br.imag,
            };
            chunk[k + half] = Complex {
                real: a.real - br.real,
                imag: a.imag - br.imag,
            };
            // FFT_COMPLEX_SELFMUL(root, unit)
            root = Complex {
                real: root.real * unit.real - root.imag * unit.imag,
                imag: root.real * unit.imag + root.imag * unit.real,
            };
        }
    }
}

/// All butterfly stages: step = 2, 4, 8, ..., 1 << logsize (C: `fft_raw`).
fn fft_raw(x: &mut [Complex], logsize: u32) {
    if logsize <= 2 {
        // step = 2 stage (and step = 4 when logsize == 2)
        if logsize >= 1 {
            butterfly(x, 2);
        }
        if logsize >= 2 {
            butterfly(x, 4);
        }
        return;
    }
    butterfly(x, 2);
    butterfly(x, 4);
    let size = 1usize << logsize;
    let mut step: usize = 8;
    while step <= size {
        butterfly(x, step);
        step *= 2;
    }
}

/// Reciprocal unit root e^(-i * 2*pi / N), computed in f64 then cast to f32
/// to match C's double-precision cos/sin (C: `FFT_COMPLEX_UNITROOT_RECIP`).
fn unitroot_recip(n: usize) -> Complex {
    let angle = 2.0 * std::f64::consts::PI / n as f64;
    Complex {
        real: angle.cos() as f32,
        imag: -angle.sin() as f32,
    }
}
