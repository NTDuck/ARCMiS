//! Rust translation of the C FFT library.
//!
//! Provides an in-place and out-of-place radix-2 FFT over `f32` complex
//! numbers, mirroring the original C implementation.

/// Complex number with single-precision components, mirroring
/// `struct fft_complex { float real; float imag; }`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub real: f32,
    pub imag: f32,
}

impl Complex {
    pub const fn new(real: f32, imag: f32) -> Self {
        Complex { real, imag }
    }

    /// `FFT_COMPLEX_ADD`
    pub fn add(a: Self, b: Self) -> Self {
        Complex {
            real: a.real + b.real,
            imag: a.imag + b.imag,
        }
    }

    /// `FFT_COMPLEX_SUB`
    pub fn sub(a: Self, b: Self) -> Self {
        Complex {
            real: a.real - b.real,
            imag: a.imag - b.imag,
        }
    }

    /// `FFT_COMPLEX_MUL`
    pub fn mul(a: Self, b: Self) -> Self {
        Complex {
            real: a.real * b.real - a.imag * b.imag,
            imag: a.real * b.imag + a.imag * b.real,
        }
    }

    /// `FFT_COMPLEX_SELFMUL`
    pub fn selfmul(&mut self, z: Self) {
        let r = self.real * z.real - self.imag * z.imag;
        let i = self.real * z.imag + self.imag * z.real;
        self.real = r;
        self.imag = i;
    }

    /// `FFT_COMPLEX_SETONE`
    pub const fn one() -> Self {
        Complex {
            real: 1.0,
            imag: 0.0,
        }
    }

    /// `FFT_COMPLEX_UNITROOT_RECIP`: reciprocal of the unit root of
    /// `z^N = 1`, i.e. `e^(-i * 2 * pi / N)`.
    pub fn unitroot_recip(n: u32) -> Self {
        let angle = 2.0 * std::f32::consts::PI / n as f32;
        Complex {
            real: angle.cos(),
            imag: -angle.sin(),
        }
    }
}

/// Number of bits in `usize`, mirroring `INTBITS(size_t)`.
const INTBITS: u32 = std::mem::size_of::<usize>() as u32 * 8;

/// `fft_clz`: count leading zeros of a non-zero `usize`.
#[inline]
fn fft_clz(n: usize) -> u32 {
    n.leading_zeros()
}

/// `next_reversed_n`: advance to the next bit-reversed index.
#[inline]
fn next_reversed_n(reversed_n: usize, shift: u32) -> usize {
    let mut reversed_n = reversed_n << shift;
    let count_leading_ones = fft_clz(!reversed_n);
    // remove leading ones
    reversed_n <<= count_leading_ones;
    reversed_n |= 1usize << (INTBITS - 1);
    reversed_n >>= shift + count_leading_ones;
    reversed_n
}

/// `rader`: bit-reverse copy from `array` into `target`.
fn rader(array: &[Complex], target: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    // how many bits should be shifted to move the number to the most
    // significant bit
    let shift = INTBITS - logsize;
    let mut reversed_n = 0usize;
    for n in 0..size {
        target[reversed_n] = array[n];

        // get next reversed_n
        if n + 1 < size {
            reversed_n = next_reversed_n(reversed_n, shift);
        }
    }
}

/// `rader_inplace`: bit-reversal permutation in place.
fn rader_inplace(array: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    let shift = INTBITS - logsize;
    // nothing should be done for 0 and 0b111...11 (size - 1).
    let mut reversed_n = size >> 1;
    for n in 1..size - 1 {
        if n < reversed_n {
            array.swap(n, reversed_n);
        }

        // get next reversed_n
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// `DO_BUTTERFLY` / `fft_raw`: iterative radix-2 butterflies.
fn fft_raw(x: &mut [Complex], logsize: u32) {
    if logsize == 0 {
        return;
    }

    let size = 1usize << logsize;

    do_butterfly(x, 2, size);

    if logsize == 1 {
        // size == 2 ?
        return;
    }

    do_butterfly(x, 4, size);

    if logsize == 2 {
        // size == 4 ?
        return;
    }

    // do generic butterfly in a loop
    let mut step = 8usize;
    while step <= size {
        do_butterfly(x, step, size);
        step *= 2;
    }
}

/// One butterfly pass over the whole array with the given `step`.
fn do_butterfly(x: &mut [Complex], step: usize, size: usize) {
    let unit = Complex::unitroot_recip(step as u32);
    let half = step / 2;
    for begin in (0..size).step_by(step) {
        // i == 0, j == half
        let t = x[begin + half];
        let u = x[begin];
        x[begin] = Complex::add(u, t);
        x[begin + half] = Complex::sub(u, t);
        if half <= 1 {
            continue;
        }
        // i == 1, j == half + 1
        let mut root = unit;
        let t = Complex::mul(root, x[begin + half + 1]);
        let u = x[begin + 1];
        x[begin + 1] = Complex::add(u, t);
        x[begin + half + 1] = Complex::sub(u, t);
        for (i, j) in (2..half).enumerate().map(|(k, i)| (i, half + 2 + k)) {
            root.selfmul(unit);
            let t = Complex::mul(root, x[begin + j]);
            let u = x[begin + i];
            x[begin + i] = Complex::add(u, t);
            x[begin + j] = Complex::sub(u, t);
        }
    }
}

/// `fft`: out-of-place FFT. `x` is the input, `X` the output,
/// `logsize` is `log2(size)`.
#[allow(non_snake_case)]
pub fn fft(x: &[Complex], X: &mut [Complex], logsize: u32) {
    rader(x, X, logsize);
    fft_raw(X, logsize);
}

/// `fft_inplace`: in-place FFT.
pub fn fft_inplace(x: &mut [Complex], logsize: u32) {
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
