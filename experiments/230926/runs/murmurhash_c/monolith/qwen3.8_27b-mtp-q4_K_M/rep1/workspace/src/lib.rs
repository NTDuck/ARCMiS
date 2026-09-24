//! `murmurhash` - murmurhash
//!
//! copyright (c) 2014-2025 joseph werle <joseph.werle@gmail.com>

/// Version of the murmurhash crate.
pub const MURMURHASH_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns a murmur hash of `key` based on `seed`
/// using the MurmurHash3 algorithm.
pub fn murmurhash(key: &[u8], seed: u32) -> u32 {
    const C1: u32 = 0xcc9e2d51;
    const C2: u32 = 0x1b873593;
    const R1: u32 = 15;
    const R2: u32 = 13;
    const M: u32 = 5;
    const N: u32 = 0xe6546b64;

    let mut h: u32;
    let mut k: u32;
    let len = key.len();
    let body = len - (len % 4); // chunk length * 4

    h = seed;

    // for each 4 byte chunk of `key`
    for chunk in key[..body].chunks_exact(4) {
        // next 4 byte chunk of `key` (little endian extract)
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

    // remainder
    let tail = &key[body..]; // tail - last 3 bytes
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

/// Returns a murmur hash of `key` based on `seed`
/// using the MurmurHash3 algorithm.
pub fn murmurhash_str(key: &str, seed: u32) -> u32 {
    murmurhash(key.as_bytes(), seed)
}
