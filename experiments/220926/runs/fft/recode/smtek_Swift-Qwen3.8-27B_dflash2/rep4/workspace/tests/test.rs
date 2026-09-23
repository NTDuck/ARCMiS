//! Port of `tests/test.c`: 8-element alternating +/-1 input,
//! `fft_inplace(data, 3)`, exact `f32` equality on the real parts.

use fft::fft_inplace;
use fft::Complex;

#[test]
fn test_inplace() {
    let mut data = [
        Complex { re: 1.0, im: 0.0 },
        Complex { re: -1.0, im: 0.0 },
        Complex { re: 1.0, im: 0.0 },
        Complex { re: -1.0, im: 0.0 },
        Complex { re: 1.0, im: 0.0 },
        Complex { re: -1.0, im: 0.0 },
        Complex { re: 1.0, im: 0.0 },
        Complex { re: -1.0, im: 0.0 },
    ];
    let expected = [0.0, 0.0, 0.0, 0.0, 8.0, 0.0, 0.0, 0.0];

    fft_inplace(&mut data, 3);
    for i in 0..data.len() {
        assert_eq!(data[i].re, expected[i]);
    }
}
