use std::fmt;

/// Complex number with float components (matches C's fft_complex_t)
#[derive(Debug, Clone, Copy)]
pub struct FftComplex {
    pub real: f32,
    pub imag: f32,
}

impl FftComplex {
    /// Create from real and imaginary parts
    pub fn from_real(real: f32, imag: f32) -> Self {
        Self { real, imag }
    }

    /// Create a unit root at position N (z^N = 1)
    pub fn unit_root(N: usize) -> Self {
        let angle = -2.0 * std::f64::consts::PI / N as f64;
        Self {
            real: (std::f64::consts.cos(angle)).round() as f32,
            imag: (std::f64::consts.sin(angle)).round() as f32,
        }
    }

    /// Create a single-element complex number [1.0, 0.0]
    pub fn one() -> Self {
        Self { real: 1.0, imag: 0.0 }
    }

    /// Create a zero complex number
    pub fn zero() -> Self {
        Self { real: 0.0, imag: 0.0 }
    }

    /// Multiply two complex numbers (conjugate multiplication)
    pub fn mul(self, other: Self) -> Self {
        Self {
            real: self.real * other.real - self.imag * other.imag,
            imag: self.real * other.imag + self.imag * other.real,
        }
    }

    /// Multiply by unit root
    pub fn selfmul(self, unit: Self) -> Self {
        Self {
            real: self.real * unit.real - self.imag * unit.imag,
            imag: self.real * unit.imag + self.imag * unit.real,
        }
    }

    /// Copy another complex number
    pub fn copy(self) -> Self {
        Self { real: self.real, imag: self.imag }
    }

    /// Swap with another complex number
    pub fn swap(self, other: Self) -> (Self, Self) {
        let tmp = self;
        self = other;
        other = tmp;
        (self, other)
    }

    /// Set to [1.0, 0.0]
    pub fn set_one(self) -> Self {
        Self { real: 1.0, imag: 0.0 }
    }

    /// Add two complex numbers
    pub fn add(self, other: Self) -> Self {
        Self {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }

    /// Subtract two complex numbers
    pub fn sub(self, other: Self) -> Self {
        Self {
            real: self.real - other.real,
            imag: self.imag - other.imag,
        }
    }

    /// Display for debugging
    pub fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} + {}i)", self.real, self.imag)
    }
}

/// Convert FftComplex to/from array of f32 (matching C's struct layout)
impl FftComplex {
    /// Convert to array of f32 [real, imag]
    pub fn as_array(self) -> [f32; 2] {
        [self.real, self.imag]
    }

    /// Convert from array of f32
    pub fn from_array(arr: [f32; 2]) -> Self {
        Self { real: arr[0], imag: arr[1] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_complex_arithmetic() {
        let a = FftComplex::from_real(1.0, 0.0);
        let b = FftComplex::from_real(0.0, 1.0);

        let sum = a.add(b);
        assert!((sum - a).real.is_small() && (sum - a).imag.is_small());

        let diff = a.sub(b);
        assert!(diff == a);

        let neg = a.mul(FftComplex::from_real(-1.0, 0.0));
        assert!(neg.real.is_small() && neg.imag.is_small());

        let unit = FftComplex::unit_root(8);
        let scaled = a.selfmul(unit);
        assert!(scaled.real.is_small() && scaled.imag.is_small());

        let zero = FftComplex::zero();
        let copied = FftComplex::copy(a);
        assert!(copied == a);

        let (a, b) = FftComplex::swap(a, b);
        assert!(a != b);
        assert!(a == a);
        assert!(b == b);
    }

    #[test]
    fn test_unit_root() {
        let u1 = FftComplex::unit_root(1);
        assert!(u1.real.is_small() && u1.imag.is_small());

        let u2 = FftComplex::unit_root(2);
        assert!((u2 - FftComplex::one()).is_small());

        let u8 = FftComplex::unit_root(8);
        let u4 = FftComplex::unit_root(4);
        let u2 = FftComplex::unit_root(2);

        let mut x = vec![FftComplex::from_real(1.0, 0.0)];
        x.push(FftComplex::from_real(0.0, 1.0));
        x.push(FftComplex::from_real(0.0, 0.0));

        let y = x.iter().map(|c| FftComplex::from_array(c.as_array())).collect();
        let result = fft_inplace(&mut y, 3);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_radix_2_fft() {
        // Simple test: transform of [1, 0], [0, 1], [0, 0] should produce
        // [1, 0], [-0.707, 0.707], [-0.707, -0.707]
        let data = vec![
            FftComplex::from_real(1.0, 0.0),
            FftComplex::from_real(0.0, 1.0),
            FftComplex::from_real(0.0, 0.0),
        ];
        let mut result = data.clone();
        fft_inplace(&mut result, 2);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_radix_4_fft() {
        let data = vec![
            FftComplex::from_real(1.0, 0.0),
            FftComplex::from_real(0.0, 1.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
        ];
        let mut result = data.clone();
        fft_inplace(&mut result, 2);
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_radix_8_fft() {
        let data = vec![
            FftComplex::from_real(1.0, 0.0),
            FftComplex::from_real(0.0, 1.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
        ];
        let mut result = data.clone();
        fft_inplace(&mut result, 3);
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_radix_16_fft() {
        let data = vec![
            FftComplex::from_real(1.0, 0.0),
            FftComplex::from_real(0.0, 1.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            // rest zeros
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
        ];
        let mut result = data.clone();
        fft_inplace(&mut result, 4);
        assert_eq!(result.len(), 16);
    }

    #[test]
    fn test_radix_32_fft() {
        let data = vec![
            FftComplex::from_real(1.0, 0.0),
            FftComplex::from_real(0.0, 1.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            // rest zeros
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
            FftComplex::from_real(0.0, 0.0),
        ];
        let mut result = data.clone();
        fft_inplace(&mut result, 5);
        assert_eq!(result.len(), 32);
    }
}
