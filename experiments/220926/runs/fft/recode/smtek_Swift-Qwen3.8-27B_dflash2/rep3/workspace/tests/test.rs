//! Mirrors C `tests/test.c`: 8-point in-place FFT of alternating 1/-1.

use fft::{fft, fft_inplace, Complex};

fn alternating_data() -> [Complex; 8] {
    [
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
    ]
}

#[test]
fn test_8_point_inplace() {
    let mut data = alternating_data();
    fft_inplace(&mut data, 3);
    let expected_real = [0.0, 0.0, 0.0, 0.0, 8.0, 0.0, 0.0, 0.0];
    for i in 0..8 {
        assert_eq!(data[i].real, expected_real[i]);
    }
}

#[test]
fn test_8_point_out_of_place() {
    let data = alternating_data();
    let mut out = [Complex { real: 0.0, imag: 0.0 }; 8];
    fft(&data, &mut out, 3);
    let expected_real = [0.0, 0.0, 0.0, 0.0, 8.0, 0.0, 0.0, 0.0];
    for i in 0..8 {
        assert_eq!(out[i].real, expected_real[i]);
    }
}
