//! ulidgen — generate ULIDs (Universally Unique Lexicographically Sortable
//! Identifiers).
//!
//! Faithful port of `src/ulid.c` / `src/ulid.h`.
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Crockford base32 alphabet (no I, L, O, U).
/// Port of `static const char *b32alphabet` in src/ulid.c.
pub const B32_ALPHABET: [u8; 32] = *b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into a caller-owned 27-byte buffer (26 chars + NUL).
///
/// Port of `void ulidgen_r(char ulid[27])` from src/ulid.c.
///
/// The buffer is caller-owned and persistent: on a same-millisecond call the
/// 16-char random part (bytes 10..26) is incremented in place based on the
/// previous contents; otherwise it is re-randomized from 16 entropy bytes.
pub fn ulidgen_r(buf: &mut [u8; 27]) {
    buf[26] = 0;

    // clock_gettime(CLOCK_REALTIME) → SystemTime::now()
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch");
    let mut t = now.as_secs() * 1000 + now.subsec_nanos() as u64 / 1_000_000;

    // Encode the 40-bit millisecond timestamp into bytes 0..10,
    // tracking whether all 10 chars are unchanged from the previous call.
    let mut same = true;
    for i in (0..10).rev() {
        let c = B32_ALPHABET[(t % 32) as usize];
        if buf[i] != c {
            same = false;
        }
        buf[i] = c;
        t /= 32;
    }

    if same {
        // Same millisecond: increment the random part in place.
        let mut i = 25;
        while i >= 10 {
            if buf[i] == b'Z' {
                buf[i] = b'0';
                i -= 1;
                continue;
            }
            if B32_ALPHABET.contains(&buf[i]) {
                buf[i] += 1;
                return;
            }
            break;
        }
        // All wrapped (or not in the alphabet): wait a bit and retry,
        // which re-randomizes once the timestamp has advanced.
        // nanosleep(&(struct timespec){ 0, 1234567 }, NULL)
        std::thread::sleep(Duration::from_nanos(1_234_567));
        return ulidgen_r(buf);
    }

    // getentropy(ulid + 10, 16) → getrandom::getrandom; abort() on failure.
    let mut rnd = [0u8; 16];
    if getrandom::getrandom(&mut rnd).is_err() {
        std::process::abort();
    }
    for i in 0..16 {
        buf[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize];
    }
}

/// Convenience wrapper: generate one ULID and return it as a 26-char String.
pub fn ulid() -> String {
    let mut buf = [0u8; 27];
    ulidgen_r(&mut buf);
    String::from_utf8_lossy(&buf[..26]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Port of tests/test.c:test_ulid_length — ULID is 26 chars.
    #[test]
    fn test_ulid_length() {
        assert_eq!(ulid().len(), 26);
    }

    /// Port of tests/test.c:test_ulid_structure — all chars in the Crockford
    /// alphabet. (Enabled here; it was commented out in the C test main.)
    #[test]
    fn test_ulid_structure() {
        let s = ulid();
        assert!(s.bytes().all(|b| B32_ALPHABET.contains(&b)));
    }

    /// Port of tests/test.c:test_ulid_uniqueness — two consecutive ULIDs differ.
    #[test]
    fn test_ulid_uniqueness() {
        assert_ne!(ulid(), ulid());
    }

    /// Port of tests/test.c:test_ulid_sortability — after a 1.5 ms sleep the
    /// second ULID sorts after the first.
    #[test]
    fn test_ulid_sortability() {
        let a = ulid();
        std::thread::sleep(Duration::from_micros(1500));
        let b = ulid();
        assert!(a < b);
    }
}
