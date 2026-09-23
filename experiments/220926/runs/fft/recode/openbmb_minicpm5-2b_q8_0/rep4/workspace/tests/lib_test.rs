//! Unit tests for the FFT library.

#[cfg(test)]
mod tests {
    use super::*;

    // Test data from the C source test program
    const DATA: &[FftComplex] = &[
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
        FftComplex { real: 1.0, imag: 0.0 },
        FftComplex { real: -1.0, imag: 0.0 },
    ];

    const EXPECTED_OUTPUT: &[FftComplex] = &[
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 8.0, imag: 0.0 },
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 0.0, imag: 0.0 },
        FftComplex { real: 0.0, imag: 0.0 },
    ];

    // Test: fft_inplace with small array (size=8, logsize=3)
    #[test]
    fn test_fft_inplace_small() {
        let mut data = DATA.to_vec();
        fft_inplace(&mut data, 3);
        let expected = EXPECTED_OUTPUT.to_vec();
        assert_eq!(data, expected);
    }

    // Test: fft with out-of-place transform
    #[test]
    fn test_fft_out_of_place() {
        let mut data = DATA.to_vec();
        let output = fft(&data, 3);
        let expected = EXPECTED_OUTPUT.to_vec();
        assert_eq!(output, expected);
    }

    // Test: basic arithmetic operations
    #[test]
    fn test_basic_ops() {
        let a = FftComplex::from(2.0, 1.0);
        let b = FftComplex::from(1.0, 0.0);
        let c = FftComplex::from(1.0, 1.0);

        let sum = a + b;
        assert_eq!(sum.real, 3.0);
        assert_eq!(sum.imag, 1.0);

        let diff = a - b;
        assert_eq!(diff.real, 1.0);
        assert_eq!(diff.imag, 1.0);

        let prod = a * c;
        assert!((prod.real - 1.0).abs() < 1e-10);
        assert!((prod.imag - 3.0).abs() < 1e-10);
    }

    // Test: unit root computation
    #[test]
    fn test_unit_root() {
        let unit = FftComplex::unit_root(8);
        let exp = FftComplex::unit_root(8);
        assert!((unit.real - exp.real).abs() < 1e-10);
        assert!((unit.imag - exp.imag).abs() < 1e-10);
    }

    // Test: self-multiply with twiddle
    #[test]
    fn test_self_mul_twiddle() {
        let n = 8;
        let unit = FftComplex::unit_root(4);
        let self_num = FftComplex::from(1.0, 1.0);
        let result = self_num.self_mul(unit);
        assert!((result.real + 1.0).abs() < 1e-10);
        assert!((result.imag + 1.0).abs() < 1e-10);
    }

    // Test: edge cases
    #[test]
    fn test_edge_cases() {
        let empty = vec![FftComplex::zero()];
        let result = fft(&empty, 0);
        assert!(result.is_empty());

        let size1 = vec![FftComplex::one()];
        let result = fft(&size1, 0);
        assert_eq!(result.len(), 1);

        let size2 = vec![FftComplex::zero(), FftComplex::zero()];
        let result = fft(&size2, 1);
        assert_eq!(result.len(), 2);
    }

    // Test: performance sanity check
    #[test]
    fn test_performance() {
        let size = 1024;
        let data = (0..size).map(|i| FftComplex::from(1.0, 0.0)).collect::<Vec<_>>();

        fft_inplace(&mut data, usize::log2(size) as usize);

        let magnitudes = data.iter().map(|x| x.real * x.real + x.imag * x.imag).collect::<Vec<_>>();
        let max_mag = magnitudes.iter().fold(f64::NEG_INFINITY, |m, v| m.max(v));
        let min_mag = magnitudes.iter().fold(f64::NEG_INFINITY, |m, v| m.min(v));
        assert!((max_mag - min_mag).abs() < 1e-10, "Magnitude spread too large: {} vs {}", max_mag, min_mag);
    }

    // Test: large array (stress test)
    #[test]
    fn test_large_array() {
        let size = 1024;
        let data = (0..size).map(|i| FftComplex::from(1.0, 0.0)).collect();

        fft_inplace(&mut data, usize::log2(size) as usize);

        let magnitudes = data.iter().map(|x| x.real * x.real + x.imag * x.imag).collect::<Vec<_>>();
        let avg_mag = magnitudes.iter().sum::<f64>() / magnitudes.len() as f64;
        let expected_avg = (size as f64) / magnitudes.len() as f64;
        assert!((avg_mag - expected_avg).abs() < 1e-10, "Average magnitude off: {} vs {}", avg_mag, expected_avg);
    }

    // Test: random data consistency
    #[test]
    fn test_random_data() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let size = 256;
        let data = (0..size).map(|i| FftComplex::from(rng.gen_range(-10.0..10.0), rng.gen_range(-10.0..10.0))).collect();

        let mut result = vec![FftComplex::zero(); size];
        fft_inplace(&mut result, usize::log2(size) as usize);

        let inv_result = fft_inplace(&mut result, usize::log2(size) as usize);
        assert_eq!(result, data);
    }
}
