//! Rust translation of the C FFT library (radix-2 iterative Cooley-Tukey
//! with O(1) bit-reversal permutation).
//!
//! Public API:
//! - `Complex` — f32 complex number (replaces `fft_complex_t`).
//! - `fft(x, out, logsize)` — out-of-place FFT (replaces C `fft`).
//! - `fft_inplace(x, logsize)` — in-place FFT (replaces C `fft_inplace`).

pub mod complex;
pub mod fft;

pub use complex::Complex;
pub use fft::{fft, fft_inplace};
