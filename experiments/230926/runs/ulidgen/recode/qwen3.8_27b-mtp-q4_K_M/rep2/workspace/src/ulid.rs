//! Core ULID generation — port of `src/ulid.c` (`ulidgen_r`).
//!
//! Buffer-reuse contract (same as the C original): the caller reuses the
//! same 27-byte buffer across calls so that a same-millisecond repeat can be
//! detected and the random part is incremented in place instead of
//! re-randomized. Do not change the in-out API.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Crockford base-32 alphabet (C `b32alphabet`).
pub const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Fill `ulid` (26 chars + NUL, mirroring the C `char[27]`) with a new ULID.
///
/// The previous contents are used to detect a same-millisecond repeat and to
/// increment the random part in place, exactly like the C original.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    // NUL terminator, as in C.
    ulid[26] = 0;

    // Millisecond timestamp since the UNIX epoch.
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Encode the 10 most significant base-32 digits. `same` starts true and
    // is cleared on any digit mismatch (C semantics).
    let mut same = true;
    let mut t = t;
    for i in (0..10).rev() {
        let d = B32_ALPHABET[(t % 32) as usize];
        if ulid[i] != d {
            ulid[i] = d;
            same = false;
        }
        t /= 32;
    }

    if same {
        // Same-millisecond repeat: increment the random part in place,
        // right to left, wrapping 'Z' -> '0'.
        let buf = &mut ulid[10..];
        let mut i = 15usize;
        loop {
            if buf[i] == b'Z' {
                buf[i] = b'0';
            } else if B32_ALPHABET.contains(&buf[i]) {
                // Advance to the next alphabet character and stop.
                let pos = B32_ALPHABET.iter().position(|&c| c == buf[i]).unwrap();
                buf[i] = B32_ALPHABET[(pos + 1) % 32];
                return;
            } else {
                // Invalid char in the random part: fall through to re-randomize.
                break;
            }
            if i == 0 {
                // All 16 chars were 'Z': sleep and recurse (C behavior).
                std::thread::sleep(Duration::from_nanos(1_234_567));
                ulidgen_r(ulid);
                return;
            }
            i -= 1;
        }
    }

    // Fresh random part (same==false, or a corrupt random part).
    // Keep the deliberate `% 32` modulo bias — do not add rejection sampling.
    let mut rnd = [0u8; 16];
    getrandom::fill(&mut rnd).unwrap_or_else(|_| std::process::abort());
    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[rnd[i] as usize % 32];
    }
}
