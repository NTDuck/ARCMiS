//! Port of tests/test.c: 8-point in-place FFT of [1,-1,1,-1,1,-1,1,-1].

use fft::{fft_inplace, FftComplex};

#[test]
fn test_inplace() {
    let mut data = [
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
    ];

    fft_inplace(&mut data, 3);

    let output = [
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 8.0, imag: 0.0 },
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 0.0, imag: 0.0 },
    ];

    for i in 0..8 {
        assert_eq!(data[i].real, output[i].real);
    }
}
