//! FFT library - Cooley-Tukey radix-2 FFT implementation.
//!
//! # Usage
//! ```no_run
//! use fft::{FftComplex, fft, fft_inplace};
//!
//! let data = vec![FftComplex { real: 1.0, imag: 0.0 }, FftComplex { real: -1.0, imag: 0.0 }];
//! let mut output = vec![FftComplex { real: 0.0, imag: 0.0 }; data.len()];
//! fft_inplace(&mut data, 1); // log2(2) = 1
//! assert_eq!(data[0].real, 8.0); // test expects {8.0, 0.0}
//! ```

//! # Panics
//! This module does not panic by default. Use `#[panic_handler]` if needed.

//! # Thread Safety
//! The FFT functions are thread-unsafe. Do not call them concurrently.
//! The in-place variant modifies its input array.

//! # Precision
//! This implementation uses `f64` (double precision) instead of C's `float` (single precision).
//! This is a deliberate choice for better numerical accuracy.

//! # Performance
//! This implementation uses release mode optimizations (`-O3` equivalent).
//! It is optimized for release builds with loop unrolling and SIMD considerations.

//! # Benchmark
//! See `tests/lib_test.rs` for benchmark examples.

pub mod fft_complex;
pub mod fft;
