use fft::{fft, fft_inplace, Complex};

#[test]
fn test_inplace() {
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

    fft_inplace(&mut data, 3);
    for i in 0..data.len() {
        assert_eq!(data[i].real, output[i].real);
    }
}

#[test]
fn test_out_of_place() {
    let data = [
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

    let mut x = [Complex::new(0.0, 0.0); 8];
    fft(&data, &mut x, 3);
    for i in 0..data.len() {
        assert_eq!(x[i].real, output[i].real);
    }
}
