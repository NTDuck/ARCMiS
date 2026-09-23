#![allow(unused)]

use std::fmt;

/// Represents a complex number with float real and imaginary parts.
#[derive(Debug, Clone, Copy)]
pub struct FftComplex {
    pub real: f32,
    pub imag: f32,
}

impl FftComplex {
    /// Create a new zero complex number.
    pub fn zero() -> Self {
        Self { real: 0.0, imag: 0.0 }
    }

    /// Create a new complex number with given real and imaginary parts.
    pub fn new(real: f32, imag: f32) -> Self {
        Self { real, imag }
    }

    /// Create a unit root at angle -2π/N.
    /// This is e^(-i * 2π / N), the N-th root of unity.
    pub fn unit_root_reciprocal(n: usize) -> Self {
        Self {
            real: cos(2.0 * std::f32::consts::PI() / n as f32),
            imag: -sin(2.0 * std::f32::consts::PI() / n as f32),
        }
    }

    /// Add two complex numbers.
    pub fn add(self, other: Self) -> Self {
        Self {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }

    /// Subtract two complex numbers.
    pub fn sub(self, other: Self) -> Self {
        Self {
            real: self.real - other.real,
            imag: self.imag - other.imag,
        }
    }

    /// Multiply two complex numbers (conjugate multiplication).
    pub fn mul(self, other: Self) -> Self {
        Self {
            real: self.real * other.real - self.imag * other.imag,
            imag: self.real * other.imag + self.imag * other.real,
        }
    }

    /// Self-multiply: conjugate the complex number.
    pub fn selfmul(self) -> Self {
        Self {
            real: self.real,
            imag: -self.imag,
        }
    }

    /// Copy another complex number into this one.
    pub fn copy_into(self) -> Self {
        Self { real: self.real, imag: self.imag }
    }

    /// Set this to a unit root at angle -2π/N.
    pub fn set_unit_root_reciprocal(mut self, n: usize) {
        self.real = cos(2.0 * std::f32::consts::PI() / n as f32);
        self.imag = -sin(2.0 * std::f32::consts::PI() / n as f32);
    }

    /// Create a complex number with real=1.0, imag=0.0.
    pub fn one() -> Self {
        Self { real: 1.0, imag: 0.0 }
    }
}

impl fmt::Display for FftComplex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.2}, {:.2})", self.real, self.imag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_complex_operations() {
        let a = FftComplex::new(3.0, 4.0);
        let b = FftComplex::new(1.0, 2.0);

        assert_eq!(a.add(b).real, 4.0);
        assert_eq!(a.add(b).imag, 6.0);
        assert_eq!(a.sub(b).real, 2.0);
        assert_eq!(a.sub(b).imag, 2.0);
        assert_eq!(a.mul(b).real, -7.0);
        assert_eq!(a.mul(b).imag, 13.0);
    }

    #[test]
    fn test_unit_root_reciprocal() {
        let n = 4;
        let unit = FftComplex::unit_root_reciprocal(n);
        // Verify: unit^4 = 1
        let power = unit.mul(unit).mul(unit).mul(unit);
        assert!((power.real).abs() < 1e-10);
        assert!((power.imag).abs() < 1e-10);
    }

    #[test]
    fn test_selfmul() {
        let x = FftComplex::new(3.0, 4.0);
        let y = x.selfmul();
        assert_eq!(y.real, 3.0);
        assert_eq!(y.imag, -4.0);
    }

    #[test]
    fn test_display() {
        let c = FftComplex::new(1.0, 0.0);
        assert_eq!(format!("{}", c), "(1.00, 0.00)");
    }
}
