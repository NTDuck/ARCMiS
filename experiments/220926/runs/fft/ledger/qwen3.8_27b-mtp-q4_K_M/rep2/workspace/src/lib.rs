//! A radix-2 FFT library translated from C.

/// A complex number with single-precision components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub real: f32,
    pub imag: f32,
}

impl Complex {
    #[inline]
    pub fn new(real: f32, imag: f32) -> Self {
        Complex { real, imag }
    }

    #[inline]
    pub const fn one() -> Self {
        Complex { real: 1.0, imag: 0.0 }
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

    #[inline]
    pub fn unit_root_recip(n: u32) -> Complex {
        let angle = 2.0 * std::f32::consts::PI / n as f32;
        Complex {
            real: angle.cos(),
            imag: -angle.sin(),
        }
    }
}

/// Number of bits in a `usize`.
const INTBITS: u32 = std::mem::size_of::<usize>() as u32 * 8;

/// Count leading zeros of a nonzero `usize`.
#[inline]
fn fft_clz(n: usize) -> u32 {
    usize::BITS - n.leading_zeros()
}

#[inline]
fn next_reversed_n(reversed_n: usize, shift: u32) -> usize {
    let mut reversed_n = reversed_n << shift;
    let count_leading_ones = fft_clz(!reversed_n);
    reversed_n <<= count_leading_ones; // remove leading ones
    reversed_n |= 1usize << (INTBITS - 1);
    reversed_n >>= shift + count_leading_ones;
    reversed_n
}

fn rader(array: &[Complex], target: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    let shift = INTBITS - logsize;
    let mut reversed_n = 0usize;
    for n in 0..size {
        target[reversed_n] = array[n];
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

fn rader_inplace(array: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    let shift = INTBITS - logsize;
    let mut reversed_n = size >> 1;
    for n in 1..size - 1 {
        if n < reversed_n {
            array.swap(n, reversed_n);
        }
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

#[inline]
fn do_butterfly(x: &mut [Complex], step: usize) {
    let unit = Complex::unit_root_recip(step as u32);
    let half = step / 2;
    for p in x.chunks_mut(step) {
        let t = p[half];
        let u = p[0];
        p[0] = u.add(t);
        p[half] = u.sub(t);
        if half <= 1 {
            continue;
        }
        let mut root = unit;
        let t = root.mul(p[half + 1]);
        let u = p[1];
        p[1] = u.add(t);
        p[half + 1] = u.sub(t);
        let mut i = 2;
        let mut j = half + 2;
        while i < half {
            root = root.mul(unit);
            let t = root.mul(p[j]);
            let u = p[i];
            p[i] = u.add(t);
            p[j] = u.sub(t);
            i += 1;
            j += 1;
        }
    }
}

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

    let mut step = 8usize;
    while step <= 1usize << logsize {
        do_butterfly(x, step);
        step *= 2;
    }
}

/// Compute the FFT of `x` into `X`. `logsize` is the base-2 logarithm of the
/// length of the arrays.
pub fn fft(x: &[Complex], X: &mut [Complex], logsize: u32) {
    assert_eq!(x.len(), 1usize << logsize);
    assert_eq!(X.len(), 1usize << logsize);
    rader(x, X, logsize);
    fft_raw(X, logsize);
}

/// Compute the FFT of `x` in place. `logsize` is the base-2 logarithm of the
/// length of the slice.
pub fn fft_inplace(x: &mut [Complex], logsize: u32) {
    assert_eq!(x.len(), 1usize << logsize);
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
