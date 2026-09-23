use fft::Complex;

#[test]
fn test_fft_inplace() {
    let mut data = [
        Complex::new(1.0, 0.0),
        Complex::new(-1.0, 0.0),
        Complex::new(1.0, 0.0),
        Complex::new(-1.0, 0.0),
        Complex::new(1.0, 0.0),
        Complex::new(-1.0, 0.0),
        Complex::new(1.0, 0.0),
        Complex::new(-1.0, 0.0),
    ];

    let output = [
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(8.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
        Complex::new(0.0, 0.0),
    ];

    fft::fft_inplace(&mut data, 3);
    for i in 0..data.len() {
        assert_eq!(data[i].real, output[i].real);
    }
    println!("Test passed");
}
