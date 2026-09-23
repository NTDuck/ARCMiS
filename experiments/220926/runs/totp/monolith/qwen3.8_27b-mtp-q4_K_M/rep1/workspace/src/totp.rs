//! TOTP algorithms: SHA1, HMAC-SHA1, HOTP, TOTP, base32 decoding.
//!
//! Translated from the C implementation by Sijmen J. Mulder.

/// `TOTP_OK`
pub const TOTP_OK: i32 = 0;
/// `TOTP_EBOUNDS` - argument out-of-bounds, overflow
pub const TOTP_EBOUNDS: i32 = 1;

/// convert u32 to 4 bytes, big-endian
pub fn unpack32(x: u32, a: &mut [u8; 4]) {
    a[0] = (x >> 24) as u8;
    a[1] = (x >> 16) as u8;
    a[2] = (x >> 8) as u8;
    a[3] = x as u8;
}

/// convert u64 to 8 bytes, big-endian
pub fn unpack64(x: u64, a: &mut [u8; 8]) {
    unpack32((x >> 32) as u32, &mut a[0..4].try_into().unwrap());
    unpack32(x as u32, &mut a[4..8].try_into().unwrap());
}

/// convert 4 bytes to u32, big-endian
pub fn pack32(a: &[u8]) -> u32 {
    (a[0] as u32) << 24 | (a[1] as u32) << 16 | (a[2] as u32) << 8 | a[3] as u32
}

/// FIPS 180-3 2.2.2
pub fn rotl(x: u32, n: u32) -> u32 {
    x.rotate_left(n)
}

/// FIPS 180-3
///
/// Parameters:
///   buf  - input buffer, clobbered, allow for 128 bytes extra
///   len  - len of buf, in bytes
///   cap  - capacity of buf, in bytes
///   hash - output buffer
///
/// Returns TOTP_OK on success, TOTP_* on error
pub fn sha1(buf: &mut [u8], len: usize, cap: usize, hash: &mut [u8; 20]) -> i32 {
    // 4.2.1, use k[t/20]
    const K: [u32; 4] = [0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC, 0xCA62C1D6];

    // 5.1.1 (padding)

    // add 1 byte for stop bit, 8 for length, pad to 64 bytes
    if len > usize::MAX - 9 - 63 {
        return TOTP_EBOUNDS;
    }
    let new_len = (len + 9 + 63) / 64 * 64; // ceil len+9 to 64 multiple
    if new_len > cap {
        return TOTP_EBOUNDS;
    }

    for i in len..new_len {
        buf[i] = 0;
    }
    buf[len] = 1 << 7;
    let mut tail = [0u8; 8];
    unpack64(len as u64 * 8, &mut tail);
    buf[new_len - 8..new_len].copy_from_slice(&tail);

    // 5.3.1
    let mut h: [u32; 5] = [
        0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0,
    ];

    // 6.1.2
    for i in 0..new_len / 64 {
        let mut w = [0u32; 80];
        for t in 0..16 {
            w[t] = pack32(&buf[i * 64 + t * 4..i * 64 + t * 4 + 4]);
        }
        for t in 16..80 {
            w[t] = rotl(w[t - 3] ^ w[t - 8] ^ w[t - 14] ^ w[t - 16], 1);
        }

        let (mut a, mut b, mut c, mut d, mut e) = (
            h[0], h[1], h[2], h[3], h[4],
        );

        for t in 0..80u32 {
            // 4.1.1 (f function)
            let f = if t < 20 {
                (b & c) ^ (!b & d)
            } else if t < 40 {
                b ^ c ^ d
            } else if t < 60 {
                (b & c) ^ (b & d) ^ (c & d)
            } else {
                b ^ c ^ d
            };

            let T = a.rotate_left(5).wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(K[(t / 20) as usize])
                .wrapping_add(w[t as usize]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = T;
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    for i in 0..5 {
        let mut out = [0u8; 4];
        unpack32(h[i], &mut out);
        hash[i * 4..i * 4 + 4].copy_from_slice(&out);
    }

    TOTP_OK
}

/// RFC 2104
///
/// Parameters:
///   key   - zero-padded key
///   data  - up to 64 bytes of data
///   len   - len of data
///   hash  - output buffer
///
/// Returns TOTP_OK on success, TOTP_* on error
pub fn hmac_sha1(key: &[u8; 64], data: &[u8], len: usize, hash: &mut [u8; 20]) -> i32 {
    let mut buf = [0u8; 196];

    if len > 64 {
        return TOTP_EBOUNDS;
    }

    for i in 0..64 {
        buf[i] = key[i] ^ 0x36;
    }
    buf[64..64 + len].copy_from_slice(&data[..len]);
    sha1(&mut buf, 64 + len, buf.len(), hash);

    for i in 0..64 {
        buf[i] = key[i] ^ 0x5C;
    }
    buf[64..64 + 20].copy_from_slice(&hash);
    sha1(&mut buf, 64 + 20, buf.len(), hash);

    TOTP_OK
}

/// RFC 4226
///
/// Parameters
///   key      - zero-padded shared secret
///   counter
///
/// Returns HOTP code or -1 on error.
pub fn hotp(key: &[u8; 64], counter: u64) -> i32 {
    let mut data = [0u8; 8];
    let mut hash = [0u8; 20];
    unpack64(counter, &mut data);
    hmac_sha1(key, &data, 8, &mut hash);

    let trunc = pack32(&hash[(hash[19] & 0xF) as usize..(hash[19] & 0xF) as usize + 4])
        & 0x7FFFFFFF;

    (trunc % 1_000_000) as i32
}

/// RFC 6238
///
/// Parameters
///   key  - zero-padded shared secret
///   time - Unix time
///
/// Returns HOTP code or -1 on error.
pub fn totp(key: &[u8; 64], time: u64) -> i32 {
    hotp(key, time / 30)
}

/// RFC 4648
///
/// Parameters:
///   s   - input base32 string, multiple of 8 length
///   buf - output buffer, at least 5 bytes for every 8 in s
///   cap - capacity of buf, in bytes
///
/// Returns number of bytes written to buf, or 0 for invalid base32.
pub fn from_base32(s: &str, buf: &mut [u8], cap: usize) -> usize {
    let s_len = s.len();
    let bytes: Vec<u8> = s.as_bytes().to_vec();

    if s_len % 8 != 0 {
        return 0;
    }
    if cap < (s_len + 1) / 8 * 5 {
        return 0;
    }

    let mut i = 0usize;
    while i * 8 < s_len {
        let mut v = [0u8; 8];
        for j in 0..8 {
            let c = bytes[i * 8 + j];
            let val = if c == b'=' {
                0
            } else if c >= b'A' && c <= b'Z' {
                c - b'A'
            } else if c >= b'a' && c <= b'z' {
                c - b'a'
            } else if c >= b'2' && c <= b'7' {
                c - b'2' + 26
            } else {
                return 0;
            };
            v[j] = val;
        }

        buf[i * 5] = (v[0] << 3) | (v[1] >> 2);
        buf[i * 5 + 1] = (v[1] << 6) | (v[2] << 1) | (v[3] >> 4);
        buf[i * 5 + 2] = (v[3] << 4) | (v[4] >> 1);
        buf[i * 5 + 3] = (v[4] << 7) | (v[5] << 2) | (v[6] >> 3);
        buf[i * 5 + 4] = (v[6] << 5) | v[7];

        if bytes[i * 8 + 2] == b'=' {
            return i * 5 + 1;
        }
        if bytes[i * 8 + 4] == b'=' {
            return i * 5 + 2;
        }
        if bytes[i * 8 + 5] == b'=' {
            return i * 5 + 3;
        }
        if bytes[i * 8 + 7] == b'=' {
            return i * 5 + 4;
        }
        i += 1;
    }

    i * 5
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_hex(a: &[u8], len: usize) -> String {
        let mut s = String::new();
        for i in 0..len {
            s.push_str(&format!("{:02x}", a[i]));
        }
        s
    }

    #[test]
    fn test_pack() {
        let mut a = [0u8; 8];

        unpack32(0x12345678, &mut a[0..4].try_into().unwrap());
        assert_eq!(a[0], 0x12);
        assert_eq!(a[1], 0x34);
        assert_eq!(a[2], 0x56);
        assert_eq!(a[3], 0x78);

        unpack64(0x123456789ABCDEF0, &mut a);
        assert_eq!(a[0], 0x12);
        assert_eq!(a[1], 0x34);
        assert_eq!(a[2], 0x56);
        assert_eq!(a[3], 0x78);
        assert_eq!(a[4], 0x9A);
        assert_eq!(a[5], 0xBC);
        assert_eq!(a[6], 0xDE);
        assert_eq!(a[7], 0xF0);

        assert_eq!(pack32(&a[0..4]), 0x12345678);
    }

    #[test]
    fn test_sha1() {
        let mut buf = [0u8; 512];
        let mut hash = [0u8; 20];

        sha1(&mut buf, 0, buf.len(), &mut hash);
        assert_eq!(
            to_hex(&hash, 20),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );

        let abc = b"abc";
        let mut buf = [0u8; 512];
        buf[..3].copy_from_slice(abc);
        sha1(&mut buf, 3, buf.len(), &mut hash);
        assert_eq!(
            to_hex(&hash, 20),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );

        let fox = b"The quick brown fox jumps over the lazy dog";
        let mut buf = [0u8; 512];
        buf[..fox.len()].copy_from_slice(fox);
        sha1(&mut buf, fox.len(), buf.len(), &mut hash);
        assert_eq!(
            to_hex(&hash, 20),
            "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"
        );
    }

    #[test]
    fn test_hmac_sha1() {
        let mut key = [0u8; 64];
        let mut text = [0u8; 64];
        let mut hash = [0u8; 20];

        // RFC 2202
        for i in 0..20 {
            key[i] = 0xAA;
        }
        for i in 0..50 {
            text[i] = 0xDD;
        }

        hmac_sha1(&key, &text, 50, &mut hash);
        assert_eq!(
            to_hex(&hash, 20),
            "125d7342b9ac11cd91a39af48aa17b4f63f175d3"
        );
    }

    #[test]
    fn test_hotp() {
        // Appendix D
        let mut secret = [0u8; 64];
        let appendix = [
            0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
            0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
            0x37, 0x38, 0x39, 0x30,
        ];
        secret[..20].copy_from_slice(&appendix);

        assert_eq!(hotp(&secret, 0), 755224);
        assert_eq!(hotp(&secret, 1), 287082);
        assert_eq!(hotp(&secret, 2), 359152);
    }

    #[test]
    fn test_from_base32() {
        let mut buf = [0u8; 10];

        assert_eq!(from_base32("MZxw6===", &mut buf, buf.len()), 3);
        assert_eq!(from_base32("MZxw6YQ=", &mut buf, buf.len()), 4);
        assert_eq!(from_base32("MZxw6YTB", &mut buf, buf.len()), 5);
        assert_eq!(from_base32("MZxw6YTBOI======", &mut buf, buf.len()), 6);

        assert_eq!(&buf[..6], b"foobar");
    }
}
