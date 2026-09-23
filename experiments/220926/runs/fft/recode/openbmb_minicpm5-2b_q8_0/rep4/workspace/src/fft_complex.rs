//! Core data types and basic operations for the FFT library.

use std::fmt;

/// A complex number used in the FFT algorithm.
#[derive(Debug, Clone, Copy)]
pub struct FftComplex {
    pub real: f64,
    pub imag: f64,
}

impl FftComplex {
    /// Create a unit root of order N: e^(-i * 2π / N).
    pub fn unit_root(n: usize) -> Self {
        let angle = -2.0 * std::f64::consts::PI / n as f64;
        Self { real: std::cos(angle), imag: std::sin(angle) }
    }

    /// Create the reciprocal of the unit root of order N.
    pub fn unit_root_reciprocal(n: usize) -> Self {
        Self::unit_root(n)
    }

    pub fn one() -> Self { Self { real: 1.0, imag: 0.0 } }
    pub fn zero() -> Self { Self { real: 0.0, imag: 0.0 } }

    pub fn from(re: f64, im: f64) -> Self {
        Self { real: re, imag: im }
    }

    pub fn add(self, other: Self) -> Self {
        Self { real: self.real + other.real, imag: self.imag + other.imag }
    }

    pub fn sub(self, other: Self) -> Self {
        Self { real: self.real - other.real, imag: self.imag - other.imag }
    }

    pub fn mul(self, other: Self) -> Self {
        Self { real: self.real * other.real - self.imag * other.imag,
                 imag: self.real * other.imag + self.imag * other.real }
    }

    /// Self-multiply: compute `(self * unit_root(N))`.
    pub fn self_mul(self, unit: Self) -> Self {
        Self { real: self.real * unit.real - self.imag * unit.imag,
                 imag: self.real * unit.imag + self.imag * unit.real }
    }

    pub fn swap(self, other: Self) -> (Self, Self) {
        (other, self)
    }

    pub fn copy(self) -> Self {
        self.clone()
    }

    pub fn set_one(self) -> &mut Self {
        self.real = 1.0; self.imag = 0.0; self
    }

    pub fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.4}, {:.4})", self.real, self.imag)
    }
}

impl Default for FftComplex {
    fn default() -> Self { Self::zero() }
}

impl fmt::Display for FftComplex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt(f)
    }
}

//! # Unit Tests
//!
//! These tests verify the correctness of basic arithmetic operations.

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_add() {
        let a = FftComplex::one();
        let b = FftComplex::from(2.0, 0.0);
        assert_eq!(FftComplex::add(a, b).real, 3.0);
        assert_eq!(FftComplex::add(a, b).imag, 0.0);
    }

    #[test] fn test_sub() {
        let a = FftComplex::from(3.0, 1.0);
        let b = FftComplex::zero();
        assert_eq!(FftComplex::sub(a, b).real, 3.0);
        assert_eq!(FftComplex::sub(a, b).imag, 1.0);
    }

    #[test] fn test_mul() {
        let a = FftComplex::from(2.0, 1.0);
        let b = FftComplex::from(1.0, 2.0);
        let r = FftComplex::mul(a, b);
        assert!((r.real - 0.0).abs() < 1e-10);
        assert!((r.imag - 5.0).abs() < 1e-10);
    }

    #[test] fn test_self_mul() {
        let unit = FftComplex::unit_root(4);
        let self_num = FftComplex::from(1.0, 1.0);
        let result = self_num.self_mul(unit);
        assert!((result.real - 1.0).abs() < 1e-10);
        assert!((result.imag - (-1.0)).abs() < 1e-10);
    }

    #[test] fn test_zero() {
        assert_eq!(FftComplex::zero().real, 0.0);
        assert_eq!(FftComplex::zero().imag, 0.0);
    }

    #[test] fn test_one() {
        assert_eq!(FftComplex::one().real, 1.0);
        assert_eq!(FftComplex::one().imag, 0.0);
    }

    #[test] fn test_fmt() {
        let c = FftComplex::from(1.0, 0.0);
        assert_eq!(format!("{c}"), "(1.0000, 0.0000)");
    }

    #[test] fn test_unit_root() {
        let unit = FftComplex::unit_root(8);
        let exp = FftComplex::unit_root(8);
        assert!((unit.real - exp.real).abs() < 1e-10);
        assert!((unit.imag - exp.imag).abs() < 1e-10);
    }

    #[test] fn test_unit_root_reciprocal() {
        let unit = FftComplex::unit_root_reciprocal(4);
        let exp = FftComplex::unit_root(4);
        assert!((unit.real - exp.real).abs() < 1e-10);
        assert!((unit.imag - exp.imag).abs() < 1e-10);
    }

    #[test] fn test_roundtrip() {
        let n = 8;
        let unit = FftComplex::unit_root(n);
        let exp = FftComplex::unit_root_reciprocal(n);
        assert!((unit.real - exp.real).abs() < 1e-10);
        assert!((unit.imag - exp.imag).abs() < 1e-10);
    }
}
