//! TOTP algorithms: SHA1 (FIPS 180-3), HMAC-SHA1 (RFC 2104),
//! HOTP (RFC 4226), TOTP (RFC 6238), and base32 decoding (RFC 4648).

/// Operation succeeded.
pub const TOTP_OK: i32 = 0;
/// Argument out-of-bounds, overflow.
pub const TOTP_EBOUNDS: i32 = 1;

/// Convert u32 to 4 bytes, big-endian.
#[inline]
pub fn unpack32(x: u32, a: &mut [u8; 4]) {
    a[0] = (x >> 24) as u8;
    a[1] = (x >> 16) as u8;
    a[2] = (x >> 8) as u8;
    a[3] = x as u8;
}

/// Convert u64 to 8 bytes, big-endian.
#[inline]
pub fn unpack64(x: u64, a: &mut [u8; 8]) {
    let mut hi = [0u8; 4];
    let mut lo = [0u8; 4];
    unpack32((x >> 32) as u32, &mut hi);
    unpack32(x as u32, &mut lo);
    a.copy_from_slice(&[hi[0], hi[1], hi[2], hi[3], lo[0], lo[1], lo[2], lo[3]]);
}

/// Convert 4 bytes to u32, big-endian.
#[inline]
pub fn pack32(a: &[u8; 4]) -> u32 {
    (a[0] as u32) << 24 | (a[1] as u32) << 16 | (a[2] as u32) << 8 | a[3] as u32
}

/// FIPS 180-3 2.2.2
#[inline]
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
/// Returns TOTP_OK on success, TOTP_EBOUNDS on error.
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
    let mut len_bytes = [0u8; 8];
    unpack64((len as u64) * 8, &mut len_bytes);
    buf[new_len - 8..new_len].copy_from_slice(&len_bytes);

    // 5.3.1
    let mut h: [u32; 5] = [
        0x67452301,
        0xEFCDAB89,
        0x98BADCFE,
        0x10325476,
        0xC3D2E1F0,
    ];

    // 6.1.2
    for i in 0..new_len / 64 {
        let mut w = [0u32; 80];
        for t in 0..16 {
            let mut chunk = [0u8; 4];
            chunk.copy_from_slice(&buf[i * 64 + t * 4..i * 64 + t * 4 + 4]);
            w[t] = pack32(&chunk);
        }
        for t in 16..80 {
            w[t] = rotl(w[t - 3] ^ w[t - 8] ^ w[t - 14] ^ w[t - 16], 1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];

        for t in 0..80 {
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

            let T = rotl(a, 5).wrapping_add(f).wrapping_add(e).wrapping_add(K[t / 20]).wrapping_add(w[t]);
            e = d;
            d = c;
            c = rotl(b, 30);
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
        let mut chunk = [0u8; 4];
        unpack32(h[i], &mut chunk);
        hash[i * 4..i * 4 + 4].copy_from_slice(&chunk);
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
/// Returns TOTP_OK on success, TOTP_EBOUNDS on error.
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
    buf[64..64 + 20].copy_from_slice(hash);
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

    let offset = hash[19] & 0xF;
    let mut chunk = [0u8; 4];
    chunk.copy_from_slice(&hash[offset as usize..offset as usize + 4]);
    let trunc = pack32(&chunk) & 0x7FFFFFFF;

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
    let s = s.as_bytes();
    let len = s.len();

    if len % 8 != 0 {
        return 0;
    }
    if cap < (len + 1) / 8 * 5 {
        return 0;
    }

    let mut i = 0usize;
    while i * 8 < len {
        let mut v = [0u8; 8];
        for j in 0..8 {
            let c = s[i * 8 + j];
            v[j] = if c == b'=' {
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
        }

        buf[i * 5] = (v[0] << 3) | (v[1] >> 2);
        buf[i * 5 + 1] = (v[1] << 6) | (v[2] << 1) | (v[3] >> 4);
        buf[i * 5 + 2] = (v[3] << 4) | (v[4] >> 1);
        buf[i * 5 + 3] = (v[4] << 7) | (v[5] << 2) | (v[6] >> 3);
        buf[i * 5 + 4] = (v[6] << 5) | v[7];

        if s[i * 8 + 2] == b'=' {
            return i * 5 + 1;
        }
        if s[i * 8 + 4] == b'=' {
            return i * 5 + 2;
        }
        if s[i * 8 + 5] == b'=' {
            return i * 5 + 3;
        }
        if s[i * 8 + 7] == b'=' {
            return i * 5 + 4;
        }
        i += 1;
    }

    i * 5
}
