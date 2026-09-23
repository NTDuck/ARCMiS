//! Integration test mirroring `tests/test.c`.

use fft::{fft_inplace, FftComplex};

#[test]
fn test_8point() {
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

    for i in 0..data.len() {
        assert_eq!(data[i].real, [0.0, 0.0, 0.0, 0.0, 8.0, 0.0, 0.0, 0.0][i]);
    }
}
