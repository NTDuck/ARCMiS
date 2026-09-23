//! libc definitions for freestanding targets (WASM, GBA).
//!
//! In the Rust translation these are thin wrappers over the standard
//! library, kept for parity with the C `std.{c,h}` pair.

/// Fill a byte buffer with a value (like C `memset`).
pub fn memset(s: &mut [u8], c: u8) {
    s.fill(c);
}

/// Copy bytes from `src` to `dst` (like C `memcpy`).
pub fn memcpy(dst: &mut [u8], src: &[u8]) {
    dst[..src.len()].copy_from_slice(src);
}

/// Length of a NUL-terminated byte string (like C `strlen`).
pub fn strlen(s: &[u8]) -> usize {
    match s.iter().position(|&b| b == 0) {
        Some(i) => i,
        None => s.len(),
    }
}
