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

    let mut output = [Complex::new(0.0, 0.0); 8];

    fft(&data, &mut output, 3);
    assert_eq!(output[4].real, 8.0);
    for i in 0..8 {
        if i != 4 {
            assert_eq!(output[i].real, 0.0);
        }
    }
}
