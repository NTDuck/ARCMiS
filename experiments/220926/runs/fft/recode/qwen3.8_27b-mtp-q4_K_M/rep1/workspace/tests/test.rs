//! Rust port of `tests/test.c`.
//!
//! Runs `fft_inplace` on an 8-point alternating +1/-1 signal and asserts
//! the DC bin equals 8.

use fft::{Complex, fft_inplace, fft};

#[test]
fn test_inplace_dc_bin() {
    let mut data: Vec<Complex> = vec![
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
    ];

    let output: Vec<Complex> = vec![
        Complex { real: 0.0, imag: 0.0 },
        Complex { real: 0.0, imag: 0.0 },
        Complex { real: 0.0, imag: 0.0 },
        Complex { real: 0.0, imag: 0.0 },
        Complex { real: 8.0, imag: 0.0 },
        Complex { real: 0.0, imag: 0.0 },
        Complex { real: 0.0, imag: 0.0 },
        Complex { real: 0.0, imag: 0.0 },
    ];

    fft_inplace(&mut data, 3);
    for i in 0..data.len() {
        assert_eq!(data[i].real, output[i].real);
    }
}

/// `fft` (out-of-place) must produce the same spectrum as `fft_inplace`.
#[test]
fn test_fft_out_of_place_matches_inplace() {
    let input: Vec<Complex> = vec![
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
    ];

    let mut inplace = input.clone();
    fft_inplace(&mut inplace, 3);

    let mut out_of_place = vec![Complex { real: 0.0, imag: 0.0 }; 8];
    fft(&input, &mut out_of_place, 3);

    for i in 0..8 {
        assert_eq!(out_of_place[i].real, inplace[i].real);
        assert_eq!(out_of_place[i].imag, inplace[i].imag);
    }
    // DC bin of the alternating signal is 8.
    assert_eq!(out_of_place[4].real, 8.0);
    assert_eq!(out_of_place[4].imag, 0.0);
}

/// `rader` (out-of-place bit-reversal) is exercised through the public
/// `fft` API: the full 8-point spectrum of a known signal must match the
/// direct DFT definition.
#[test]
fn test_fft_full_spectrum_matches_dft() {
    let input: Vec<Complex> = vec![
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: 2.0, imag: 0.0 },
        Complex { real: 3.0, imag: 0.0 },
        Complex { real: 4.0, imag: 0.0 },
        Complex { real: 5.0, imag: 0.0 },
        Complex { real: 6.0, imag: 0.0 },
        Complex { real: 7.0, imag: 0.0 },
        Complex { real: 8.0, imag: 0.0 },
    ];

    let n = 8usize;
    let mut out = vec![Complex { real: 0.0, imag: 0.0 }; n];
    fft(&input, &mut out, 3);

    // Direct DFT: X[k] = sum_{t=0}^{n-1} x[t] * e^{-i 2 pi k t / n}
    for k in 0..n {
        let mut re = 0.0f32;
        let mut im = 0.0f32;
        for t in 0..n {
            let angle = 2.0 * std::f32::consts::PI * (k * t) as f32 / n as f32;
            re += input[t].real * angle.cos() - input[t].imag * angle.sin();
            im += input[t].real * angle.sin() + input[t].imag * angle.cos();
        }
        assert!((out[k].real - re).abs() < 1e-3, "bin {} real {} vs {}", k, out[k].real, re);
        assert!((out[k].imag - im).abs() < 1e-3, "bin {} imag {} vs {}", k, out[k].imag, im);
    }
    // DC bin is the sum of the input = 36.
    assert!((out[0].real - 36.0).abs() < 1e-3);
    assert!((out[0].imag).abs() < 1e-3);
}
