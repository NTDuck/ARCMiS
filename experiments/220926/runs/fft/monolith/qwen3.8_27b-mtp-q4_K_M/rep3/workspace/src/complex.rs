//! Complex number type and arithmetic operations, mirroring `src/fft.h`.

/// A single-precision complex number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub real: f32,
    pub imag: f32,
}

impl Complex {
    #[inline]
    pub const fn new(real: f32, imag: f32) -> Self {
        Complex { real, imag }
    }

    #[inline]
    pub fn add(self, other: Complex) -> Complex {
        Complex {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }

    #[inline]
    pub fn sub(self, other: Complex) -> Complex {
        Complex {
            real: self.real - other.real,
            imag: self.imag - other.imag,
        }
    }

    #[inline]
    pub fn mul(self, other: Complex) -> Complex {
        Complex {
            real: self.real * other.real - self.imag * other.imag,
            imag: self.real * other.imag + self.imag * other.real,
        }
    }

    /// Set to 1 + 0i.
    #[inline]
    pub fn set_one(&mut self) {
        self.real = 1.0;
        self.imag = 0.0;
    }

    /// Set to the reciprocal of the unit root of z^N = 1, i.e. e^(-i * 2 * pi / N).
    #[inline]
    pub fn set_unitroot_recip(&mut self, n: usize) {
        let angle = 2.0 * std::f32::consts::PI / n as f32;
        self.real = angle.cos();
        self.imag = -angle.sin();
    }
}
