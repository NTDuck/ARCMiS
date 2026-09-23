//! Rust translation of the C radix-2 iterative FFT library (libfft.a).
//!
//! Computes the forward DFT (twiddle factor `e^{-i*2*pi/N}`) of a
//! power-of-two sized complex sequence. The public API takes `logsize`
//! (log2 of the transform size), not the size itself.

pub mod complex;
pub mod fft;

pub use complex::FftComplex;
pub use fft::{fft, fft_inplace};
