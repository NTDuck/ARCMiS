//! Complex number type and arithmetic.
//!
//! Replaces `struct fft_complex` / `fft_complex_t` and the `FFT_COMPLEX_*`
//! macros from `src/fft.h`. Kept as `f32` to preserve bit-identical results
//! with the C implementation (the test asserts exact `==` on `f32`).

/// Complex number with `f32` components (replaces `fft_complex_t`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Complex {
    pub re: f32,
    pub im: f32,
}

impl Complex {
    /// `FFT_COMPLEX_ADD(result, a, b)` — `result = a + b`.
    pub fn add(a: Complex, b: Complex) -> Complex {
        todo!()
    }

    /// `FFT_COMPLEX_SUB(result, a, b)` — `result = a - b`.
    pub fn sub(a: Complex, b: Complex) -> Complex {
        todo!()
    }

    /// `FFT_COMPLEX_MUL(result, a, b)` — `result = a * b`.
    pub fn mul(a: Complex, b: Complex) -> Complex {
        todo!()
    }

    /// `FFT_COMPLEX_SELFMUL(self, z)` — `self = self * z` (in place).
    pub fn self_mul(&mut self, z: Complex) {
        todo!()
    }

    /// `FFT_COMPLEX_SETONE(result)` — `result = 1 + 0i`.
    pub fn set_one() -> Complex {
        todo!()
    }

    /// `FFT_COMPLEX_UNITROOT_RECIP(result, N)` —
    /// `result = e^(-i * 2 * pi / N)` (reciprocal of the N-th unit root).
    pub fn unit_root_recip(n: usize) -> Complex {
        todo!()
    }
}
