//! The algorithms: SHA1 (FIPS 180-3), HMAC-SHA1 (RFC 2104), HOTP (RFC 4226),
//! TOTP (RFC 6238), base32 decode (RFC 4648).

/// Convert u32 to 4 bytes, big-endian.
pub fn unpack32(x: u32, a: &mut [u8; 4]) {
    a[0] = (x >> 24) as u8;
    a[1] = (x >> 16) as u8;
    a[2] = (x >> 8) as u8;
    a[3] = x as u8;
}

/// Convert u64 to 8 bytes, big-endian.
pub fn unpack64(x: u64, a: &mut [u8; 8]) {
    let hi = (x >> 32) as u32;
    let lo = x as u32;
    unpack32(hi, &mut a[0..4].try_into().unwrap());
    unpack32(lo, &mut a[4..8].try_into().unwrap());
}

/// Convert 4 bytes to u32, big-endian.
pub fn pack32(a: &[u8; 4]) -> u32 {
    ((a[0] as u32) << 24) | ((a[1] as u32) << 16) | ((a[2] as u32) << 8) | (a[3] as u32)
}

/// Rotate left by n bits (FIPS 180-3 2.2.2).
pub fn rotl(x: u32, n: u32) -> u32 {
    x.rotate_left(n)
}

/// FIPS 180-3 SHA1.
///
/// C signature `sha1(buf, len, cap, hash)` clobbered `buf` in place for
/// padding; the Rust version takes the message as a slice and returns the
/// 20-byte digest.
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let len = data.len();
    let new_len = (len + 9 + 63) / 64 * 64;
    let mut buf = vec![0u8; new_len];
    buf[..len].copy_from_slice(data);
    buf[len] = 0x80;
    let bit_len = (len as u64) * 8;
    let mut len_bytes = [0u8; 8];
    unpack64(bit_len, &mut len_bytes);
    buf[new_len - 8..].copy_from_slice(&len_bytes);

    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let k = [0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC, 0xCA62C1D6];

    for chunk in buf.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = pack32(&chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..80 {
            w[i] = rotl(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
        }

        let (mut a, mut b, mut c, mut d, mut e) =
            (h[0], h[1], h[2], h[3], h[4]);

        for i in 0..80 {
            let (f, k_i) = match i {
                0..=19 => ((b & c) | ((!b) & d), k[0]),
                20..=39 => (b ^ c ^ d, k[1]),
                40..=59 => ((b & c) | (b & d) | (c & d), k[2]),
                _ => (b ^ c ^ d, k[3]),
            };
            let temp = rotl(a, 5) + f + e + k_i + w[i];
            e = d;
            d = c;
            c = rotl(b, 30);
            b = a;
            a = temp;
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    let mut hash = [0u8; 20];
    for i in 0..5 {
        unpack32(h[i], &mut hash[i * 4..i * 4 + 4].try_into().unwrap());
    }
    hash
}

/// RFC 2104 HMAC-SHA1. `key` is a 64-byte zero-padded key; `data` is up to
/// 64 bytes.
pub fn hmac_sha1(key: &[u8; 64], data: &[u8]) -> [u8; 20] {
    let mut ipad = [0u8; 64];
    let mut opad = [0u8; 64];
    for i in 0..64 {
        ipad[i] = key[i] ^ 0x36;
        opad[i] = key[i] ^ 0x5C;
    }

    let mut inner = Vec::with_capacity(64 + data.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(data);
    let inner_hash = sha1(&inner);

    let mut outer = Vec::with_capacity(64 + 20);
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_hash);
    sha1(&outer)
}

/// RFC 4226 HOTP. Returns the 6-digit code, or `None` on error.
pub fn hotp(key: &[u8; 64], counter: u64) -> Option<u32> {
    let mut counter_bytes = [0u8; 8];
    unpack64(counter, &mut counter_bytes);
    let hash = hmac_sha1(key, &counter_bytes);
    let offset = hash[19] & 0x0F;
    let code = pack32(&hash[offset as usize..offset as usize + 4].try_into().unwrap())
        & 0x7FFF_FFFF;
    Some(code % 1_000_000)
}

/// RFC 6238 TOTP. `time` is Unix time in seconds.
pub fn totp(key: &[u8; 64], time: u64) -> Option<u32> {
    hotp(key, time / 30)
}

/// RFC 4648 base32 decode.
///
/// `s` must be a multiple of 8 characters; `=` padding, upper/lower case and
/// digits 2-7 are accepted. Returns the number of bytes written to `buf`,
/// or `Err(())` for invalid base32 / insufficient capacity.
pub fn from_base32(s: &str, buf: &mut [u8]) -> Result<usize, ()> {
    let len = s.len();
    if len % 8 != 0 || buf.len() < (len + 1) / 8 * 5 {
        return Err(());
    }

    let chars: Vec<u8> = s.bytes().collect();
    for i in 0..len {
        let c = chars[i];
        if c == b'=' {
            // Padding may only appear at the end of a group.
            if i % 8 != 2 && i % 8 != 4 && i % 8 != 5 && i % 8 != 7 {
                return Err(());
            }
            return Ok(i / 8 * 5 + 1);
        }
        let v = if c >= b'A' && c <= b'Z' {
            c - b'A'
        } else if c >= b'a' && c <= b'z' {
            c - b'a' + 26
        } else if c >= b'2' && c <= b'7' {
            c - b'2' + 52
        } else {
            return Err(());
        };
        if v >= 32 {
            return Err(());
        }
        // Accumulate 8 chars into 5 bytes.
        if i % 8 == 0 {
            buf[i / 8 * 5] = 0;
        }
        buf[i / 8 * 5 + (i % 8) / 8] = buf[i / 8 * 5 + (i % 8) / 8]
            .wrapping_add(v as u8 << (5 - i % 8));
    }
    Ok(len / 8 * 5)
}
