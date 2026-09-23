//! Complex number type and complex arithmetic.
//!
//! Replaces the C `fft_complex_t` struct and the `FFT_COMPLEX_*` macros
//! from `src/fft.h`.

/// Single-precision complex number, mirroring `struct fft_complex`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FftComplex {
    pub real: f32,
    pub imag: f32,
}

impl FftComplex {
    /// `FFT_COMPLEX_ADD`: a + b.
    pub fn add(a: FftComplex, b: FftComplex) -> FftComplex {
        FftComplex {
            real: a.real + b.real,
            imag: a.imag + b.imag,
        }
    }

    /// `FFT_COMPLEX_SUB`: a - b.
    pub fn sub(a: FftComplex, b: FftComplex) -> FftComplex {
        FftComplex {
            real: a.real - b.real,
            imag: a.imag - b.imag,
        }
    }

    /// `FFT_COMPLEX_MUL`: a * b.
    pub fn mul(a: FftComplex, b: FftComplex) -> FftComplex {
        FftComplex {
            real: a.real * b.real - a.imag * b.imag,
            imag: a.real * b.imag + a.imag * b.real,
        }
    }

    /// `FFT_COMPLEX_SELFMUL`: self = self * z (in place).
    pub fn self_mul(&mut self, z: FftComplex) {
        let a = *self;
        *self = Self::mul(a, z);
    }

    /// `FFT_COMPLEX_SETONE`: {1.0, 0.0}.
    pub fn set_one() -> FftComplex {
        FftComplex {
            real: 1.0,
            imag: 0.0,
        }
    }

    /// `FFT_COMPLEX_UNITROOT_RECIP`: e^(-i * 2 * pi / n).
    pub fn unit_root_recip(n: usize) -> FftComplex {
        let angle = 2.0 * std::f32::consts::PI / n as f32;
        FftComplex {
            real: angle.cos(),
            imag: -angle.sin(),
        }
    }
}
