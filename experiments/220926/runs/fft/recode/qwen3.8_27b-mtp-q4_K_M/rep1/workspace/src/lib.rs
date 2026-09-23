//! Rust port of the C `libfft.a` iterative radix-2 FFT.
//!
//! Public API (mirrors `src/fft.h`):
//! - [`fft`]         : out-of-place FFT
//! - [`fft_inplace`] : in-place FFT
//!
//! The complex type is [`Complex`], the Rust analogue of C's
//! `struct fft_complex { float real; float imag; }`.

/// Single-precision complex number, the Rust equivalent of C's
/// `struct fft_complex { float real; float imag; }`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub real: f32,
    pub imag: f32,
}

impl Complex {
    /// `FFT_COMPLEX_ADD(result, a, b)` — returns `a + b`.
    pub fn add(a: Complex, b: Complex) -> Complex {
        Complex {
            real: a.real + b.real,
            imag: a.imag + b.imag,
        }
    }

    /// `FFT_COMPLEX_SUB(result, a, b)` — returns `a - b`.
    pub fn sub(a: Complex, b: Complex) -> Complex {
        Complex {
            real: a.real - b.real,
            imag: a.imag - b.imag,
        }
    }

    /// `FFT_COMPLEX_MUL(result, a, b)` — returns `a * b`.
    pub fn mul(a: Complex, b: Complex) -> Complex {
        Complex {
            real: a.real * b.real - a.imag * b.imag,
            imag: a.real * b.imag + a.imag * b.real,
        }
    }

    /// `FFT_COMPLEX_SELFMUL(self, z)` — `self = self * z`.
    pub fn self_mul(&mut self, z: Complex) {
        let r = self.real * z.real - self.imag * z.imag;
        let i = self.real * z.imag + self.imag * z.real;
        self.real = r;
        self.imag = i;
    }

    /// `FFT_COMPLEX_UNITROOT_RECIP(result, N)` — `e^(-i * 2 * pi / N)`.
    pub fn unitroot_recip(n: usize) -> Complex {
        let angle = 2.0 * std::f32::consts::PI / n as f32;
        Complex {
            real: angle.cos(),
            imag: -angle.sin(),
        }
    }
}

/// `next_reversed_n(reversed_n, shift)` — advance to the next bit-reversed index.
fn next_reversed_n(mut reversed_n: usize, shift: u32) -> usize {
    reversed_n <<= shift;
    let count_leading_ones = (!reversed_n).leading_zeros();
    reversed_n <<= count_leading_ones; // remove leading ones
    reversed_n |= 1usize << (usize::BITS - 1);
    reversed_n >>= shift + count_leading_ones;
    reversed_n
}

/// `rader(array, target, logsize)` — out-of-place bit-reversal permutation.
fn rader(array: &[Complex], target: &mut [Complex], logsize: usize) {
    let size = 1usize << logsize;
    // how many bits should be shift to move the number to the most significant bit
    let shift = usize::BITS - logsize as u32;
    let mut reversed_n = 0usize;
    for n in 0..size {
        target[reversed_n] = array[n];

        // get next reversed_n
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// `rader_inplace(array, logsize)` — in-place bit-reversal permutation.
fn rader_inplace(array: &mut [Complex], logsize: usize) {
    let size = 1usize << logsize;
    let shift = usize::BITS - logsize as u32;
    // nothing should be done for 0 and 0b111...11(size - 1).
    let mut reversed_n = size >> 1;
    for n in 1..size - 1 {
        if n < reversed_n {
            array.swap(n, reversed_n);
        }

        // get next reversed_n
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// `DO_BUTTERFLY(begin, end, step)` — one butterfly stage for a given `step`.
fn do_butterfly(x: &mut [Complex], step: usize) {
    let unit = Complex::unitroot_recip(step);
    let half = step / 2;
    let end = x.len();
    let mut p = 0;
    while p < end {
        // i == 0, j == half
        let t = x[p + half];
        let u = x[p];
        x[p] = Complex::add(u, t);
        x[p + half] = Complex::sub(u, t);
        if half <= 1 {
            p += step;
            continue;
        }
        // i == 1, j == half + 1
        let mut root = unit;
        let t = Complex::mul(root, x[p + half + 1]);
        let u = x[p + 1];
        x[p + 1] = Complex::add(u, t);
        x[p + half + 1] = Complex::sub(u, t);
        let mut i = 2;
        let mut j = half + 2;
        while i < half {
            root.self_mul(unit);
            let t = Complex::mul(root, x[p + j]);
            let u = x[p + i];
            x[p + i] = Complex::add(u, t);
            x[p + j] = Complex::sub(u, t);
            i += 1;
            j += 1;
        }
        p += step;
    }
}

/// `fft_raw(x, logsize)` — run all butterfly stages.
fn fft_raw(x: &mut [Complex], logsize: usize) {
    if logsize == 0 {
        return;
    }

    do_butterfly(x, 2);

    if logsize == 1 {
        // size == 2 ?
        return;
    }

    do_butterfly(x, 4);

    if logsize == 2 {
        // size == 4 ?
        return;
    }

    // do generic butterfly in a loop
    let mut step = 8usize;
    while step <= 1usize << logsize {
        do_butterfly(x, step);
        step *= 2;
    }
}

/// `fft(x, X, logsize)` — out-of-place FFT.
pub fn fft(x: &[Complex], x_out: &mut [Complex], logsize: usize) {
    rader(x, x_out, logsize);
    fft_raw(x_out, logsize);
}

/// `fft_inplace(x, logsize)` — in-place FFT.
pub fn fft_inplace(x: &mut [Complex], logsize: usize) {
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
