//! `murmurhash` - murmurhash
//!
//! copyright (c) 2014-2025 joseph werle <joseph.werle@gmail.com>

/// Library version, mirroring `MURMURHASH_VERSION` from the C header.
pub const VERSION: &str = "0.2.0";

/// Returns a murmur hash of `key` based on `seed`
/// using the MurmurHash3 algorithm.
///
/// This mirrors the C API:
///
/// ```c
/// uint32_t murmurhash (const char *key, uint32_t len, uint32_t seed);
/// ```
pub fn murmurhash(key: &[u8], seed: u32) -> u32 {
    const C1: u32 = 0xcc9e2d51;
    const C2: u32 = 0x1b873593;
    const R1: u32 = 15;
    const R2: u32 = 13;
    const M: u32 = 5;
    const N: u32 = 0xe6546b64;

    let len = key.len();
    let mut h: u32;
    let mut k: u32;
    let l = len / 4; // chunk length

    h = seed;

    let body = &key[..l * 4]; // body
    let tail = &key[l * 4..]; // tail - last 8 bytes

    // for each 4 byte chunk of `key`
    for chunk in body.chunks_exact(4) {
        // next 4 byte chunk of `key` (little endian extract, as in the C
        // implementation which applies `htole32` on big endian hosts)
        k = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);

        // encode next 4 byte chunk of `key`
        k = k.wrapping_mul(C1);
        k = k.rotate_left(R1);
        k = k.wrapping_mul(C2);

        // append to hash
        h ^= k;
        h = h.rotate_left(R2);
        h = h.wrapping_mul(M).wrapping_add(N);
    }

    k = 0;

    // remainder (fall-through switch on `len % 4`)
    if tail.len() >= 3 {
        k ^= (tail[2] as u32) << 16;
    }
    if tail.len() >= 2 {
        k ^= (tail[1] as u32) << 8;
    }
    if !tail.is_empty() {
        k ^= tail[0] as u32;
        k = k.wrapping_mul(C1);
        k = k.rotate_left(R1);
        k = k.wrapping_mul(C2);
        h ^= k;
    }

    h ^= len as u32;

    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;

    h
}

/// Convenience wrapper for hashing a `str` with a given seed.
pub fn murmurhash_str(key: &str, seed: u32) -> u32 {
    murmurhash(key.as_bytes(), seed)
}
