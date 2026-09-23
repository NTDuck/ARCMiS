//! Core FFT implementation using the iterative Cooley-Tukey algorithm.
//!
//! # Algorithm
//! This module implements a radix-2 FFT with bit-reversal permutation.
//! The algorithm processes the input in O(n log n) time where n is a power of 2.
//!
//! # Usage
//! ```no_run
//! use fft::{fft, fft_inplace, FftComplex};
//!
//! let data = vec![FftComplex { real: 1.0, imag: 0.0 }, FftComplex { real: -1.0, imag: 0.0 }];
//! let mut output = vec![FftComplex { real: 0.0, imag: 0.0 }; data.len()];
//! fft_inplace(&mut data, 1); // log2(2) = 1
//! assert_eq!(data[0].real, 8.0); // test expects {8.0, 0.0}
//! ```

use super::fft_complex::{FftComplex, FFT_COMPLEX_UNITROOT_RECIP};

/// Perform an out-of-place FFT.
///
/// Transforms the input array into the frequency domain.
/// The input is NOT modified.
///
/// # Arguments
/// * `x`: Input array of `FftComplex` values
/// * `logsize`: Log2 of the array size (must be a power of 2)
///
/// # Panics
/// If the array size is not a power of 2 or logsize is invalid.
pub fn fft(x: &[FftComplex], logsize: usize) -> Vec<FftComplex> {
    let size = 1 << logsize;
    if size == 0 {
        return vec![];
    }

    let mut result = vec![FftComplex::zero(); size];

    // Bit-reversal permutation
    fft_rader(x, &mut result, logsize);

    // Iterative FFT with step sizes
    fft_raw(&mut result, logsize);

    result
}

/// Perform an in-place FFT.
///
/// Transforms the input array in-place. The input is modified.
///
/// # Panics
/// If the array size is not a power of 2 or logsize is invalid.
pub fn fft_inplace(x: &mut [FftComplex], logsize: usize) {
    let size = 1 << logsize;
    if size == 0 {
        return;
    }

    // Bit-reversal permutation (in-place)
    fft_rader_inplace(x, logsize);

    // Iterative FFT with step sizes
    fft_raw_inplace(x, logsize);
}

// ---------------------------------------------------------------------------
// Bit-reversal permutation
// ---------------------------------------------------------------------------

/// Perform bit-reversal permutation on an out-of-place copy.
fn fft_rader(x: &[FftComplex], target: &mut [FftComplex], logsize: usize) {
    let size = 1 << logsize;
    if size == 0 {
        return;
    }

    // Compute the bit-reversal permutation
    let mut rev = 0u64;
    let mut i = 0u64;
    while i < size {
        rev ^= size;
        i += size >> 1;
    }

    // Apply bit-reversal to input
    for i in 0..size {
        if i < rev {
            let j = i;
            let t = x[j];
            x[j] = FftComplex::zero();
            x[j] = t;
        }
    }

    // Copy to target with bit-reversal
    for i in 0..size {
        target[i] = x[i];
    }
}

/// Perform in-place bit-reversal permutation.
fn fft_rader_inplace(x: &mut [FftComplex], logsize: usize) {
    let size = 1 << logsize;
    if size == 0 {
        return;
    }

    // Compute the bit-reversal permutation
    let mut rev = 0u64;
    let mut i = 0u64;
    while i < size {
        rev ^= size;
        i += size >> 1;
    }

    // In-place bit-reversal: swap elements that need to be swapped
    let mut j = 0u64;
    let mut i = 0u64;
    while i < size {
        if j < i {
            let t = x[i];
            x[i] = x[j];
            x[j] = t;
        }
        j = (j | (rev - 1)) & rev;
        i += size >> 1;
    }
}

// ---------------------------------------------------------------------------
// Core FFT computation
// ---------------------------------------------------------------------------

/// Perform the butterfly operations for a given step size.
fn do_butterfly_step(x: &mut [FftComplex], step: usize, half: usize) {
    let mut unit = FftComplex::unit_root(step);

    for i in 0..half {
        let t = x[i * step];
        let u = x[(i * step) + half];

        let t_copy = t;
        let u_copy = u;

        x[i * step] = FftComplex::add(t_copy, unit);
        x[(i * step) + half] = FftComplex::sub(t_copy, u_copy);

        // Update twiddle factor for next iteration
        unit = unit.self_mul(unit);
    }
}

/// Perform the full FFT computation.
fn fft_raw(x: &mut [FftComplex], logsize: usize) {
    if logsize == 0 {
        return;
    }

    let size = 1 << logsize;

    // Step 2: butterfly with step=2
    do_butterfly_step(x, 2, size >> 1);

    if logsize == 1 {
        return;
    }

    // Step 4: butterfly with step=4
    do_butterfly_step(x, 4, size >> 1);

    if logsize == 2 {
        return;
    }

    // Generic loop: step=8,16,32,...,size
    let mut step = 8;
    while step <= size {
        let unit = FftComplex::unit_root(step);
        do_butterfly_step(x, step, size >> 1);
        step *= 2;
    }
}

/// Perform the in-place FFT computation.
fn fft_raw_inplace(x: &mut [FftComplex], logsize: usize) {
    let size = 1 << logsize;
    if size == 0 {
        return;
    }

    // Step 2: butterfly with step=2
    do_butterfly_step(x, 2, size >> 1);

    if logsize == 1 {
        return;
    }

    // Step 4: butterfly with step=4
    do_butterfly_step(x, 4, size >> 1);

    if logsize == 2 {
        return;
    }

    // Generic loop: step=8,16,32,...,size
    let mut step = 8;
    while step <= size {
        let unit = FftComplex::unit_root(step);
        do_butterfly_step(x, step, size >> 1);
        step *= 2;
    }
}

/// Perform the butterfly operation for a given step size.
fn do_butterfly_step_inplace(x: &mut [FftComplex], step: usize, half: usize) {
    let mut unit = FftComplex::unit_root(step);

    for i in 0..half {
        let t = x[i * step];
        let u = x[(i * step) + half];

        let t_copy = t;
        let u_copy = u;

        x[i * step] = FftComplex::add(t_copy, unit);
        x[(i * step) + half] = FftComplex::sub(t_copy, u_copy);

        unit = unit.self_mul(unit);
    }
}
