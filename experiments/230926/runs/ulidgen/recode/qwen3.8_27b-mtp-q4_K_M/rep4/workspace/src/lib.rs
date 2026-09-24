//! ulidgen — ULID generation core.
//!
//! Faithful port of `src/ulid.c` (C `ulidgen_r(char ulid[27])`).
//!
//! Contract (must be preserved):
//! - `ulid` is 27 bytes: 26 Crockford-base32 chars + NUL terminator.
//! - The function is STATEFUL through the caller's buffer: the previous
//!   ULID left in the buffer is read to detect a same-millisecond call and
//!   to increment the random part in place.
//! - The random bytes are encoded with the biased `rnd[i] % 32` mapping —
//!   do NOT "fix" it to uniform base32.
//! - On full random-part overflow (all 16 chars were 'Z') the function
//!   sleeps exactly 1.234567 ms and recurses.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Crockford base32 alphabet (no I, L, O, U).
const B32: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into `ulid` (27 bytes: 26 chars + NUL), matching the C
/// `ulidgen_r(char ulid[27])`. The previous ULID left in the buffer is used
/// for the same-millisecond increment path.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    ulid[26] = 0;

    // clock_gettime(CLOCK_REALTIME) -> milliseconds since the epoch.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the UNIX epoch");
    let mut t = now.as_secs() * 1000 + now.subsec_nanos() as u64 / 1_000_000;

    // Encode the 48-bit timestamp into ulid[9..=0], comparing each new char
    // against the PRE-EXISTING buffer value to detect a same-millisecond call.
    let mut same = true;
    for i in (0..10).rev() {
        let c = B32[(t % 32) as usize];
        if ulid[i] != c {
            same = false;
        }
        ulid[i] = c;
        t /= 32;
    }

    if same {
        // Increment the 16-char random part in place: carry 'Z' -> '0' from
        // the least significant char (index 15) upwards.
        let mut i: i32 = 15;
        while i >= 0 && ulid[10 + i as usize] == b'Z' {
            ulid[10 + i as usize] = b'0';
            i -= 1;
        }
        if i < 0 {
            // Full overflow: wait 1.234567 ms and recurse (re-fetches the
            // timestamp, mirroring the C code).
            std::thread::sleep(Duration::from_nanos(1_234_567));
            return ulidgen_r(ulid);
        }
        // Bump the char to its successor in the alphabet (strchr + *++p).
        match B32.iter().position(|&c| c == ulid[10 + i as usize]) {
            Some(pos) => ulid[10 + i as usize] = B32[pos + 1],
            // Char not in the alphabet: fall through to re-randomization.
            None => same = false,
        }
    }

    if !same {
        // getentropy(rnd, 16); abort() on failure.
        let mut rnd = [0u8; 16];
        getrandom::fill(&mut rnd).expect("getentropy failed");
        // Preserve the biased `rnd[i] % 32` encoding.
        for i in 0..16 {
            ulid[10 + i] = B32[rnd[i] as usize % 32];
        }
    }
}

#[cfg(test)]
mod tests {
    //! Port of `tests/test.c` (all four tests, including the structure test
    //! that is commented out in the C `main` — keep it enabled here).

    use super::*;
    use std::thread;

    /// Helper: validate ULID structure (port of `is_valid_ulid` in tests/test.c).
    fn is_valid_ulid(ulid: &[u8]) -> bool {
        ulid.len() == 26 && ulid.iter().all(|&c| B32.contains(&c))
    }

    #[test]
    fn test_ulid_length() {
        let mut ulid = [0u8; 27];
        ulidgen_r(&mut ulid);
        assert_eq!(ulid[26], 0);
        assert_eq!(ulid[..26].iter().filter(|&&c| c != 0).count(), 26);
    }

    #[test]
    fn test_ulid_structure() {
        let mut ulid = [0u8; 27];
        ulidgen_r(&mut ulid);
        assert!(is_valid_ulid(&ulid[..26]));
    }

    #[test]
    fn test_ulid_uniqueness() {
        let mut ulid1 = [0u8; 27];
        let mut ulid2 = [0u8; 27];
        ulidgen_r(&mut ulid1);
        ulidgen_r(&mut ulid2);
        assert_ne!(&ulid1[..26], &ulid2[..26]);
    }

    #[test]
    fn test_ulid_sortability() {
        let mut ulid1 = [0u8; 27];
        let mut ulid2 = [0u8; 27];
        ulidgen_r(&mut ulid1);
        thread::sleep(Duration::from_micros(1500));
        ulidgen_r(&mut ulid2);
        assert!(ulid1[..26] < ulid2[..26]);
    }
}
