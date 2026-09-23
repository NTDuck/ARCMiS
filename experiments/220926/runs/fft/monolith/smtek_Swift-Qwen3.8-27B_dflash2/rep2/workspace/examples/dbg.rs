fn main() {
    let mut data = [
        fft::Complex::new(1.0, 0.0),
        fft::Complex::new(-1.0, 0.0),
        fft::Complex::new(1.0, 0.0),
        fft::Complex::new(-1.0, 0.0),
        fft::Complex::new(1.0, 0.0),
        fft::Complex::new(-1.0, 0.0),
        fft::Complex::new(1.0, 0.0),
        fft::Complex::new(-1.0, 0.0),
    ];
    fft::fft_inplace(&mut data, 3);
    for (i, c) in data.iter().enumerate() {
        println!("{}: {} {}", i, c.real, c.imag);
    }
}
