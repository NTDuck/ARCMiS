//! `murmurhash` - Rust port of the C `murmurhash.c` implementation.
//!
//! NOTE: the C chunk loop is written in an unusual way:
//!
//! ```c
//! l = len / 4;
//! chunks = (const uint32_t *)(d + l * 4);
//! for (i = -l; i != 0; ++i) { k = htole32(chunks[i]); ... }
//! ```
//!
//! Because `chunks` points PAST the last chunk (`d + l*4`), `chunks[-l]` is the
//! FIRST chunk and `chunks[-1]` is the LAST one: the loop visits the 4-byte
//! chunks in FORWARD order (chunk 0, chunk 1, ..., chunk l-1). The Rust port
//! reproduces exactly this behavior; the C test vectors (e.g. `"kinkajou"`
//! seed 0 -> `0xb6d99cf8`, `""` seed 1 -> `0x514e28b7`) only match with this
//! order.

/// Compute the murmurhash of `key` with the given `seed`.
///
/// Bit-identical to the C `murmurhash(key, len, seed)` implementation,
/// including the chunk visit order implied by the C loop and the
/// fallthrough remainder handling.
pub fn murmurhash(key: &[u8], seed: u32) -> u32 {
    const C1: u32 = 0xcc9e2d51;
    const C2: u32 = 0x1b873593;
    const R1: u32 = 15;
    const R2: u32 = 13;
    const M: u32 = 5;
    const N: u32 = 0xe6546b64;

    let len = key.len() as u32;
    let l = (len / 4) as usize; // chunk count

    let mut h: u32 = seed;

    // C: chunks = d + l*4; for (i = -l; i != 0; ++i) k = htole32(chunks[i]);
    // chunks[-l] == d (first chunk), chunks[-1] == d + 4l - 4 (last chunk),
    // so the chunks are visited in forward order: 0, 1, ..., l-1.
    for i in 0..l {
        // next 4 byte chunk of `key` (htole32 -> u32::from_le_bytes)
        let mut k = u32::from_le_bytes([
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

    let mut k: u32 = 0;

    // remainder (C switch fallthrough: case 3 -> case 2 -> case 1)
    let tail = &key[l * 4..];
    match len & 3 {
        3 => {
            k ^= (tail[2] as u32) << 16;
            k ^= (tail[1] as u32) << 8;
            k ^= tail[0] as u32;
            k = k.wrapping_mul(C1);
            k = k.rotate_left(R1);
            k = k.wrapping_mul(C2);
            h ^= k;
        }
        2 => {
            k ^= (tail[1] as u32) << 8;
            k ^= tail[0] as u32;
            k = k.wrapping_mul(C1);
            k = k.rotate_left(R1);
            k = k.wrapping_mul(C2);
            h ^= k;
        }
        1 => {
            k ^= tail[0] as u32;
            k = k.wrapping_mul(C1);
            k = k.rotate_left(R1);
            k = k.wrapping_mul(C2);
            h ^= k;
        }
        _ => {}
    }

    h ^= len;

    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;

    h
}

#[cfg(test)]
mod tests {
    use super::murmurhash;

    /// Port of the `t(str, seed, expected)` macro from `test.c`.
    fn t(str: &str, seed: u32, expected: u32) {
        let hash = murmurhash(str.as_bytes(), seed);
        assert_eq!(
            expected, hash,
            "['{}'] seed '{}' = '{}' (expected '{}')",
            str, seed, hash, expected
        );
    }

    #[test]
    fn test_seed_0() {
        let seed: u32 = 0;
        t("", seed, 0x00000000);
        t("0", seed, 0xd271c07f);
        t("01", seed, 0x61ec6600);
        t("012", seed, 0xec6cff8c);
        t("0123", seed, 0xd41994a0);
        t("01234", seed, 0x19d02170);
        t("2", seed, 0x0129e217);
        t("88", seed, 0x7a0040a5);

        t("asdfqwer", seed, 0xa46b5209);
        t("asdfqwerty", seed, 0xa3cfe04b);
        t("asd", seed, 0x14570c6f);

        t("Hello", seed, 0x12da77c8);
        t("Hello1", seed, 0x6357e0a6);
        t("Hello2", seed, 0xe5ce223e);

        t("hey", seed, 0x12f94418);
        t("dude", seed, 0xef0487f3);
        t("test", seed, 0xba6bd213);
        t("kinkajou", seed, 0xb6d99cf8);
    }

    #[test]
    fn test_seed_1() {
        let seed: u32 = 1;
        t("", seed, 0x514e28b7);
    }
}
