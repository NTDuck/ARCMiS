use std::arch::x86_64::clz;

/// Complex number representation matching the C source.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct fft_complex_t {
    pub real: f32,
    pub imag: f32,
}

impl fft_complex_t {
    /// Create a complex number from two floats (matching C struct layout).
    #[inline]
    pub fn new(real: f32, imag: f32) -> Self {
        Self { real, imag }
    }

    /// Add two complex numbers component-wise.
    #[inline]
    pub fn add(self, other: Self) -> Self {
        Self {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }

    /// Subtract two complex numbers component-wise.
    #[inline]
    pub fn sub(self, other: Self) -> Self {
        Self {
            real: self.real - other.real,
            imag: self.imag - other.imag,
        }
    }

    /// Multiply two complex numbers.
    #[inline]
    pub fn mul(self, other: Self) -> Self {
        Self {
            real: self.real * other.real - self.imag * other.imag,
            imag: self.real * other.imag + self.imag * other.real,
        }
    }

    /// Self-multiply (square).
    #[inline]
    pub fn selfmul(self) -> Self {
        self.mul(self)
    }

    /// Copy self.
    #[inline]
    pub fn copy(self) -> Self {
        Self {
            real: self.real,
            imag: self.imag,
        }
    }

    /// Swap real and imaginary parts.
    #[inline]
    pub fn swap(self) -> Self {
        Self {
            real: self.imag,
            imag: self.real,
        }
    }

    /// Create a complex number with real=1.0, imag=0.0.
    #[inline]
    pub fn setone() -> Self {
        Self { real: 1.0, imag: 0.0 }
    }

    /// Create a complex number with real=0.0, imag=1.0.
    #[inline]
    pub fn unitroot_recip() -> Self {
        Self { real: 0.0, imag: 1.0 }
    }
}

/// Compute bit-width of a non-negative integer.
/// Uses GCC builtin for GCC/Clang, portable fallback otherwise.
#[inline]
fn fft_clz(n: u32) -> u32 {
    if n == 0 {
        return 32;
    }
    if cfg!(target_arch = "x86_64") {
        clz(n) as u32
    } else {
        // Portable fallback: count trailing zeros via loop
        let mut count = 0;
        let mut mask = 1u32;
        while mask & n == 0 {
            count += 1;
            mask <<= 1;
        }
        count
    }
}

/// Compute next reversed index in bit-reversed order.
/// For n=8 (binary 1000), this maps:
///  0 -> 0, 1 -> 7, 2 -> 6, 3 -> 5, 4 -> 4, 5 -> 3, 6 -> 2, 7 -> 1
/// Formula: next_reversed_n(n, i) = (n >> 1) ^ i
/// This is equivalent to the standard bit-reversal permutation.
#[inline]
fn next_reversed_n(n: u32, i: u32) -> u32 {
    ((n >> 1) ^ i) as u32
}

/// Bit-reversal permutation: creates a new array with elements reversed.
fn rader(data: &mut [fft_complex_t], n: u32) {
    let mut i = 0;
    let mut j = 0;
    while i < n {
        j = next_reversed_n(n, i);
        if i == j {
            i += 1;
            continue;
        }
        let tmp = data[i].copy();
        data[i] = data[j].copy();
        data[j] = tmp;
        i += 1;
        j = next_reversed_n(n, i);
        if i == j {
            i += 1;
            continue;
        }
        let tmp = data[i].copy();
        data[i] = data[j].copy();
        data[j] = tmp;
        i += 1;
        j = next_reversed_n(n, i);
    }
}

/// In-place bit-reversal permutation. Skips n=0 and middle element.
fn rader_inplace(data: &mut [fft_complex_t], n: u32) {
    if n == 0 {
        return;
    }
    let mut i = 0;
    let mut j = 0;
    while i < n {
        j = next_reversed_n(n, i);
        if i == j {
            i += 1;
            continue;
        }
        let tmp = data[i].copy();
        data[i] = data[j].copy();
        data[j] = tmp;
        i += 1;
        j = next_reversed_n(n, i);
        if i == j {
            i += 1;
            continue;
        }
        let tmp = data[i].copy();
        data[i] = data[j].copy();
        data[j] = tmp;
        i += 1;
        j = next_reversed_n(n, i);
    }
}

/// Generic butterfly loop with unit roots for radix-2 DIT FFT.
/// Computes: t = w * A, A' = B + t
/// where w is the twiddle factor (unit root), A and B are input pairs.
/// The scaling factor 1/sqrt(N) is applied implicitly through the butterfly structure.
macro_rules! DO_BUTTERFLY {
    ($data:expr, $step:expr, $logsize:expr, $w:expr) => {
        for ($i:usize, $j:usize) in (0..$step / $step).enumerate() {
            for ($k:usize) in (0..$step / $step).enumerate() {
                let $k = $k as f32;
                let $j = $j as f32;
                let $k_lo = $k * $j;
                let $k_hi = $k_lo * $j;
                let $k_lo_lo = $k_lo * $j;
                let $k_lo_hi = $k_lo * $j;
                let $k_hi_lo = $k_hi * $j;
                let $k_hi_hi = $k_hi * $j;
                let $tmp = $data[$k_lo_lo].add($data[$k_lo_hi]);
                let $tmp = $tmp.sub($data[$k_hi_lo]);
                let $tmp = $tmp.mul($data[$k_hi_hi]);
                let $w_val = $w * $j;
                let $tmp = $tmp.add($w_val);
                $data[$k_lo_lo] = $tmp;
                $data[$k_lo_hi] = $tmp;
            }
        }
    };
}

/// Recursive-style butterfly expansion from step=8 up to power of two.
/// Expands the FFT by doubling the step size until reaching the full size.
fn fft_raw(data: &mut [fft_complex_t], n: u32, logsize: u32) {
    let mut step = 8u32;
    while step < n {
        step <<= 1;
        let half_step = step >> 1;
        let logsize = usize::log2(step);
        let w = unitroot_recip();
        let inv_sqrt_n = 1.0 / (step as f32).sqrt();
        DO_BUTTERFLY!($data, $step, $logsize, $w);
        let mut inv_sqrt_half = inv_sqrt_n.sqrt();
        let mut w = unitroot_recip();
        let mut w_val = w * half_step as f32;
        DO_BUTTERFLY!($data, $half_step, $logsize, $w);
        w = unitroot_recip();
        let mut w_val = w * half_step as f32;
        DO_BUTTERFLY!($data, $half_step, $logsize, $w);
        step <<= 1;
        logsize += 1;
    }
}

/// Full bit-reversal permutation followed by butterfly computation.
/// Wraps bit-reversal then butterfly.
fn fft(data: &mut [fft_complex_t], n: u32) {
    rader(data, n);
    fft_raw(data, n, usize::log2(n) as u32);
}

/// In-place FFT: same as fft but operates in-place.
fn fft_inplace(data: &mut [fft_complex_t], n: u32) {
    rader_inplace(data, n);
    fft_raw(data, n, usize::log2(n) as u32);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_arithmetic() {
        let a = fft_complex_t::new(3.0, 4.0);
        let b = fft_complex_t::new(1.0, 2.0);

        let sum = a.add(b);
        assert!((sum.real - 4.0).abs() < f32::EPSILON);
        assert!((sum.imag - 6.0).abs() < f32::EPSILON);

        let diff = a.sub(b);
        assert!((diff.real - 2.0).abs() < f32::EPSILON);
        assert!((diff.imag - 2.0).abs() < f32::EPSILON);

        let prod = a.mul(b);
        assert!((prod.real - (-3.0 * 1.0 + 4.0 * 2.0)).abs() < f32::EPSILON);
        assert!((prod.imag - (3.0 * 2.0 - 4.0 * 1.0)).abs() < f32::EPSILON);

        let selfmul = a.selfmul();
        assert!((selfmul.real - (-3.0 * 3.0 + 4.0 * 4.0)).abs() < f32::EPSILON);
        assert!((selfmul.imag - (3.0 * 3.0 + 4.0 * 3.0)).abs() < f32::EPSILON);

        let copy = a.copy();
        assert!(copy.real == a.real);
        assert!(copy.imag == a.imag);

        let swapped = a.swap();
        assert!((swapped.real - 4.0).abs() < f32::EPSILON);
        assert!((swapped.imag - 3.0).abs() < f32::EPSILON);

        let one = fft_complex_t::setone();
        assert!((one.real - 1.0).abs() < f32::EPSILON);
        assert!((one.imag - 0.0).abs() < f32::EPSILON);

        let inv_unitroot = fft_complex_t::unitroot_recip();
        assert!((inv_unitroot.real - 0.0).abs() < f32::EPSILON);
        assert!((inv_unitroot.imag - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fft_inplace() {
        let mut data: [fft_complex_t; 8] = vec![
            fft_complex_t::new(1.0, 0.0),
            fft_complex_t::new(0.0, 0.0),
            fft_complex_t::new(1.0, 0.0),
            fft_complex_t::new(0.0, 0.0),
            fft_complex_t::new(1.0, 0.0),
            fft_complex_t::new(0.0, 0.0),
            fft_complex_t::new(1.0, 0.0),
            fft_complex_t::new(0.0, 0.0),
        ];

        fft_inplace(&mut data, 8);

        // After FFT, the element at index 4 (middle) should contain the sum
        // of all elements (DC component)
        let middle = data[4];
        assert!((middle.real - 8.0).abs() < f32::EPSILON);
        assert!((middle.imag - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fft_complex_arithmetic() {
        let mut data: [fft_complex_t; 8] = vec![
            fft_complex_t::new(1.0, 0.0),
            fft_complex_t::new(0.0, 0.0),
            fft_complex_t::new(1.0, 0.0),
            fft_complex_t::new(0.0, 0.0),
            fft_complex_t::new(1.0, 0.0),
            fft_complex_t::new(0.0, 0.0),
            fft_complex_t::new(1.0, 0.0),
            fft_complex_t::new(0.0, 0.0),
        ];

        fft(&mut data, 8);

        // After FFT, the element at index 4 (middle) should contain the sum
        // of all elements (DC component)
        let middle = data[4];
        assert!((middle.real - 8.0).abs() < f32::EPSILON);
        assert!((middle.imag - 0.0).abs() < f32::EPSILON);
    }
}
