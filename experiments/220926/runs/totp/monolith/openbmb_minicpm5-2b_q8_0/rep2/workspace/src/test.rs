/// Tests for the TOTP implementation.
/// Mirrors test.c from the C source.

use totp::{
    hotp, hmac_sha1, pack32, pack64, from_base32, unpack64, sha1, totp, Error,
};

// ── Test: pack/unpack 32-bit integers ─────────────────────────────

#[test]
fn test_pack() {
    let a = [0x12, 0x34, 0x56, 0x78];
    let expected = 0x12345678;
    let result = pack32(a);
    assert_eq!(result, expected);
}

#[test]
fn test_pack64() {
    let a = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
    let expected = 0x123456789ABCDEF0;
    let result = pack64(a);
    assert_eq!(result, expected);
}

// ── Test: SHA-1 ──────────────────────────────────────────────────

#[test]
fn test_sha1() {
    let buf = b"da39a3ee5e6b4b0d3255bfef95601890afd80709";
    let hash = sha1(buf, 11, 20, [0u8; 20]);
    assert_eq!(hash, buf);
}

#[test]
fn test_sha1_abc() {
    let buf = b"abc";
    let hash = sha1(buf, 3, 20, [0u8; 20]);
    let expected = b"a9993e364706816aba3e25717850c26c9cd0d89d";
    assert_eq!(hash, expected);
}

#[test]
fn test_sha1_quick_brown() {
    let text = "The quick brown fox jumps over the lazy dog";
    let hash = sha1(text.as_bytes(), text.len(), 20, [0u8; 20]);
    let expected = b"2fd4e1c67a2d28fced849ee1bb76e7391b93eb12";
    assert_eq!(hash, expected);
}

// ── Test: HMAC-SHA-1 ─────────────────────────────────────────────

#[test]
fn test_hmac_sha1() {
    let key = b"0123456789abcdef0123456789abcdef";
    let text = b"hello world";
    let hash = hmac_sha1(&key, text, text.len(), [0u8; 20]);
    let expected = b"125d7342b9ac11cd91a39af48aa17b4f63f175d3";
    assert_eq!(hash, expected);
}

// ── Test: HOTP ────────────────────────────────────────────────────

#[test]
fn test_hotp() {
    let secret = [
        0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
        0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
        0x37, 0x38, 0x39, 0x30,
    ];

    assert_eq!(hotp(&secret, 0), 755224);
    assert_eq!(hotp(&secret, 1), 287082);
    assert_eq!(hotp(&secret, 2), 359152);
}

// ── Test: Base32 decoding ─────────────────────────────────────────

#[test]
fn test_from_base32() {
    let mut buf = [0u8; 10];

    // "MZxw6===": M=12, Z=25, w=22, 6=6, === padding
    // = 0x12, 0x19, 0x16, 0x06, 0x00, 0x00, 0x00
    let result = from_base32("MZxw6===", &mut buf);
    assert_eq!(result, 6);
    assert_eq!(buf[0], 0x12);
    assert_eq!(buf[1], 0x19);
    assert_eq!(buf[2], 0x16);
    assert_eq!(buf[3], 0x06);

    // "MZxw6YQ=": M=12, Z=25, w=22, 6=6, Y=25, Q=16, = padding
    let result = from_base32("MZxw6YQ=", &mut buf);
    assert_eq!(result, 6);
    assert_eq!(buf[0], 0x12);
    assert_eq!(buf[1], 0x19);
    assert_eq!(buf[2], 0x16);
    assert_eq!(buf[3], 0x06);
    assert_eq!(buf[4], 0x25);
    assert_eq!(buf[5], 0x16);

    // "MZxw6YTB": M=12, Z=25, w=22, 6=6, Y=25, T=19, B=27
    let result = from_base32("MZxw6YTB", &mut buf);
    assert_eq!(result, 5);
    assert_eq!(buf[0], 0x12);
    assert_eq!(buf[1], 0x19);
    assert_eq!(buf[2], 0x16);
    assert_eq!(buf[3], 0x19);
    assert_eq!(buf[4], 0x27);

    // "foobar" in base32
    let result = from_base32("foobar", &mut buf);
    assert_eq!(result, 5);
    assert_eq!(buf[0], 0x12);
    assert_eq!(buf[1], 0x19);
    assert_eq!(buf[2], 0x16);
    assert_eq!(buf[3], 0x19);
    assert_eq!(buf[4], 0x27);
}

// ── Test: Totp/HOTP with invalid seed ──────────────────────────────

#[test]
fn test_totp_invalid_seed() {
    let key = [0u8; 64];
    let time = 1_700_000_000;
    assert!(totp(&key, time).is_err());
}

// ── Test: Totp/HOTP with valid seed ───────────────────────────────

#[test]
fn test_totp_valid_seed() {
    let key = [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let time = 1_700_000_000;
    let expected = hotp(&key, time / 30);
    assert!(totp(&key, time).is_ok());
}

// ── Test: Bounds checking ──────────────────────────────────────

#[test]
fn test_sha1_bounds() {
    // Empty input should fail bounds check
    let result = sha1(b"", 0, 20);
    assert!(result.is_err());
}

#[test]
fn test_hmac_sha1_bounds() {
    let key = [0u8; 64];
    let data = vec![0xAA; 50];
    let result = hmac_sha1(&key, &data, data.len(), [0u8; 20]);
    assert!(result.is_err());
}
