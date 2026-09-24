//! `murmurhash` - murmurhash
//!
//! copyright (c) 2014-2025 joseph werle <joseph.werle@gmail.com>

/// Package version
pub const VERSION: &str = "0.2.0";

/// Returns a murmur hash of `key` based on `seed`
/// using the MurmurHash3 algorithm
pub fn murmurhash(key: &[u8], seed: u32) -> u32 {
    const C1: u32 = 0xcc9e2d51;
    const C2: u32 = 0x1b873593;
    const R1: u32 = 15;
    const R2: u32 = 13;
    const M: u32 = 5;
    const N: u32 = 0xe6546b64;

    let len = key.len();
    let mut h: u32 = seed;
    let mut k: u32;
    let l = len / 4; // chunk length

    // for each 4 byte chunk of `key`
    for i in 0..l {
        // next 4 byte chunk of `key` (little endian, as with htole32)
        k = u32::from_le_bytes([
            key[i * 4],
            key[i * 4 + 1],
            key[i * 4 + 2],
            key[i * 4 + 3],
        ]);

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

    // remainder
    let tail = &key[l * 4..]; // last 8 byte chunk of `key`
    if tail.len() >= 3 {
        k ^= (tail[2] as u32) << 16;
    }
    if tail.len() >= 2 {
        k ^= (tail[1] as u32) << 8;
    }
    if tail.len() >= 1 {
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
