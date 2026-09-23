//! FFT implementation, mirroring `src/fft.c`.

use crate::complex::Complex;

const INTBITS: u32 = std::mem::size_of::<usize>() as u32 * 8;

/// Number of leading zeros in `n` (n must be nonzero).
#[inline]
fn clz(n: usize) -> u32 {
    usize::leading_zeros(n)
}

/// Compute the next reversed index, mirroring `next_reversed_n`.
#[inline]
fn next_reversed_n(reversed_n: usize, shift: u32) -> usize {
    let mut r = reversed_n << shift;
    let count_leading_ones = clz(!r);
    r <<= count_leading_ones; // remove leading ones
    r |= 1 << (INTBITS - 1);
    r >> (shift + count_leading_ones)
}

/// Bit-reverse copy from `array` into `target`, mirroring `rader`.
fn rader(array: &[Complex], target: &mut [Complex], logsize: u32) {
    let size = 1usize << logsize;
    let shift = INTBITS - logsize;
    let mut reversed_n = 0usize;
    for n in 0..size {
        target[reversed_n] = array[n];
        reversed_n = next_reversed_n(reversed_n, shift);
    }
}

/// In-place bit-reversal permutation, mirroring `rader_inplace`.
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

/// One butterfly pass over `x` with the given `step`, mirroring `DO_BUTTERFLY`.
fn do_butterfly(x: &mut [Complex], step: usize) {
    let mut unit = Complex::new(0.0, 0.0);
    unit.set_unitroot_recip(step);
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

/// Raw FFT on a bit-reversed array, mirroring `fft_raw`.
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

/// Compute the FFT of `x` (length `1 << logsize`) into `X`.
pub fn fft(x: &[Complex], x_out: &mut [Complex], logsize: u32) {
    rader(x, x_out, logsize);
    fft_raw(x_out, logsize);
}

/// Compute the FFT of `x` in place.
pub fn fft_inplace(x: &mut [Complex], logsize: u32) {
    rader_inplace(x, logsize);
    fft_raw(x, logsize);
}
