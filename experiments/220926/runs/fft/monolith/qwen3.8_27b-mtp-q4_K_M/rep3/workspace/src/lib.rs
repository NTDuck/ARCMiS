//! FFT library.

pub mod complex;
pub mod fft;

pub use complex::Complex;
pub use fft::{fft, fft_inplace};
