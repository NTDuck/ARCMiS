//! ulidgen — generate ULID
//! (Universally Unique Lexicographically Sortable Identifier)
//!
//! Rust port of `src/ulid.c` / `src/ulid.h` from the public-domain C project
//! by Leah Neukirchen <leah@vuxu.org>.
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Crockford Base32 alphabet (ULID spec), most-significant digit first.
pub const B32_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID (26 Crockford-Base32 characters).
///
/// Port of `ulidgen_r(char[27])` from `src/ulid.c`. The C function detects
/// "same millisecond" by comparing against the caller-reused output buffer;
/// here the previous ULID is passed explicitly via `prev`.
///
/// - If `prev` is `Some` and its timestamp part (first 10 chars) equals the
///   current millisecond, the random part (chars 10..26) is incremented in
///   place with Crockford-Base32 carry (`'Z'` wraps to `'0'`).
/// - If the random part was all `'Z'` (overflow), sleep 1.234567 ms and retry.
/// - If any char of `prev`'s random part is not in the alphabet, fall through
///   to full re-randomization.
/// - Otherwise the random part is 16 fresh CSPRNG bytes, each encoded as
///   `B32_ALPHABET[byte % 32]`.
pub fn ulidgen(prev: Option<&str>) -> String {
    loop {
        // 1. Current millisecond timestamp (48-bit range).
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_millis() as u64;

        // 2. Encode the 48-bit timestamp into the first 10 chars,
        //    most-significant digit first.
        let mut ulid = String::with_capacity(26);
        let mut t = ms;
        for _ in 0..10 {
            ulid.push(B32_ALPHABET.as_bytes()[(t % 32) as usize] as char);
            t /= 32;
        }

        // 3. Same-millisecond detection against `prev`'s timestamp part.
        if let Some(prev) = prev {
            let prev_bytes = prev.as_bytes();
            if prev_bytes.len() >= 26 && prev_bytes[..10] == *ulid.as_bytes() {
                // 4. Increment the random part (chars 10..26) in place.
                let mut rnd: Vec<u8> = prev_bytes[10..26].to_vec();
                let mut overflow = false;
                let mut corrupt = false;

                // Walk from the last char, wrapping 'Z' -> '0' and carrying.
                let mut i = 15;
                loop {
                    let c = rnd[i] as char;
                    if c == 'Z' {
                        rnd[i] = b'0';
                        if i == 0 {
                            // Whole random part was 'Z' -> overflow.
                            overflow = true;
                            break;
                        }
                        i -= 1;
                        continue;
                    }
                    match B32_ALPHABET.find(c) {
                        Some(pos) => {
                            // Bump to the successor in the alphabet.
                            rnd[i] = B32_ALPHABET.as_bytes()[pos + 1];
                            break;
                        }
                        None => {
                            // Char not in the alphabet: corrupt buffer,
                            // fall through to full re-randomization.
                            corrupt = true;
                            break;
                        }
                    }
                }

                if !overflow && !corrupt {
                    ulid.push_str(&String::from_utf8(rnd).expect("valid utf8"));
                    return ulid;
                }
                if overflow {
                    // 5. Overflow: sleep 1.234567 ms and retry.
                    std::thread::sleep(Duration::from_nanos(1_234_567));
                    continue;
                }
                // corrupt: fall through to re-randomization below.
            }
        }

        // 6. Fresh random part: 16 CSPRNG bytes, each -> alphabet[byte % 32].
        let rnd: [u8; 16] = rand::random();
        for b in rnd {
            ulid.push(B32_ALPHABET.as_bytes()[(b % 32) as usize] as char);
        }
        return ulid;
    }
}

/// Convenience wrapper: generate a ULID with no previous value (fresh random
/// part). Equivalent to `ulidgen(None)`.
pub fn ulidgen_fresh() -> String {
    ulidgen(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Port of `is_valid_ulid` from `tests/test.c`.
    fn is_valid_ulid(ulid: &str) -> bool {
        ulid.len() == 26 && ulid.chars().all(|c| B32_ALPHABET.contains(c))
    }

    /// Port of `test_ulid_length` from `tests/test.c`.
    #[test]
    fn test_ulid_length() {
        assert_eq!(ulidgen_fresh().len(), 26);
    }

    /// Port of `test_ulid_structure` from `tests/test.c`
    /// (disabled in the C main; enabled here).
    #[test]
    fn test_ulid_structure() {
        assert!(is_valid_ulid(&ulidgen_fresh()));
    }

    /// Port of `test_ulid_uniqueness` from `tests/test.c`.
    #[test]
    fn test_ulid_uniqueness() {
        let a = ulidgen_fresh();
        let b = ulidgen(Some(&a));
        assert_ne!(a, b);
    }

    /// Port of `test_ulid_sortability` from `tests/test.c`.
    #[test]
    fn test_ulid_sortability() {
        let a = ulidgen_fresh();
        std::thread::sleep(Duration::from_micros(1500));
        let b = ulidgen(Some(&a));
        assert!(a < b);
    }
}
