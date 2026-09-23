//! Mirrors tests/test.c: 8-point in-place FFT of [1,-1,1,-1,1,-1,1,-1],
//! asserting the real parts (expected 8.0 at index 4, 0.0 elsewhere).

use fft::{fft_inplace, Complex};

#[test]
fn test_inplace_8() {
    let mut data = [
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
        Complex { real: 1.0, imag: 0.0 },
        Complex { real: -1.0, imag: 0.0 },
    ];
    let output = [
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
