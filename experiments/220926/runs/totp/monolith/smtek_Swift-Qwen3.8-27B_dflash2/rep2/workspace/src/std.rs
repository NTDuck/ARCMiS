//! libc definitions for freestanding targets (WASM, GBA).
//!
//! In the C original these are provided as real functions for
//! freestanding builds. In Rust the standard library already provides
//! these, so this module only re-exports them for API compatibility.

use std::mem;

pub const SIZE_MAX: usize = mem::SIZE_MAX;

/// Set the first `n` bytes of `s` to `c`.
pub fn memset(s: &mut [u8], c: u8, n: usize) {
    s[..n].fill(c);
}

/// Copy `n` bytes from `src` to `dst`.
pub fn memcpy(dst: &mut [u8], src: &[u8], n: usize) {
    dst[..n].copy_from_slice(&src[..n]);
}

/// Length of a NUL-terminated byte string.
pub fn strlen(s: &[u8]) -> usize {
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}
