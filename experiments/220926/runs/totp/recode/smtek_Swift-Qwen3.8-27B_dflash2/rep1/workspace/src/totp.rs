//! The algorithms: SHA-1 (FIPS 180-3), HMAC-SHA1 (RFC 2104), HOTP (RFC 4226),
//! TOTP (RFC 6238), base32 decode (RFC 4648).

/// `TOTP_OK` return code (C: `enum { TOTP_OK, ... }`).
pub const TOTP_OK: i32 = 0;
/// `TOTP_EBOUNDS` return code: argument out-of-bounds / overflow.
pub const TOTP_EBOUNDS: i32 = 1;

/// convert u32 to 4 bytes, big-endian
#[inline]
pub fn unpack32(x: u32, a: &mut [u8; 4]) {
    todo!()
}

/// convert u64 to 8 bytes, big-endian
#[inline]
pub fn unpack64(x: u64, a: &mut [u8; 8]) {
    todo!()
}

/// convert 4 bytes to u32, big-endian
#[inline]
pub fn pack32(a: &[u8; 4]) -> u32 {
    todo!()
}

/// FIPS 180-3 2.2.2
#[inline]
pub fn rotl(x: u32, n: u32) -> u32 {
    todo!()
}

/// FIPS 180-3.
///
/// `buf` is clobbered in place and must allow for up to 128 bytes of extra
/// headroom (`cap >= ceil((len+9)/64)*64`). Returns `TOTP_OK` on success,
/// `TOTP_EBOUNDS` on error.
pub fn sha1(buf: &mut [u8], len: usize, cap: usize, hash: &mut [u8; 20]) -> i32 {
    todo!()
}

/// RFC 2104.
///
/// `key` is a zero-padded 64-byte key, `data` is up to 64 bytes.
/// Returns `TOTP_OK` on success, `TOTP_EBOUNDS` on error.
pub fn hmac_sha1(key: &[u8; 64], data: &[u8], len: usize, hash: &mut [u8; 20]) -> i32 {
    todo!()
}

/// RFC 4226.
///
/// Returns the HOTP code, or -1 on error.
pub fn hotp(key: &[u8; 64], counter: u64) -> i32 {
    todo!()
}

/// RFC 6238.
///
/// Returns the TOTP code, or -1 on error.
pub fn totp(key: &[u8; 64], time: u64) -> i32 {
    todo!()
}

/// RFC 4648.
///
/// `s` is a base32 string whose length is a multiple of 8. Returns the number
/// of bytes written to `buf`, or 0 for invalid base32 / insufficient capacity.
pub fn from_base32(s: &str, buf: &mut [u8], cap: usize) -> usize {
    todo!()
}
