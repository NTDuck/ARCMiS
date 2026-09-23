//! Core ULID generation — faithful port of `src/ulid.c`.
//!
//! A ULID is a 26-character Crockford base32 string: 48-bit millisecond
//! timestamp (10 chars) + 80 random bits (16 chars). The function is
//! stateful via its argument buffer: it reads the previous ULID to detect
//! the same-millisecond case and increment the random part in place.

use std::time::{SystemTime, UNIX_EPOCH};

/// Crockford base32 alphabet (no I, L, O, U).
pub const B32_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into `ulid` (26 chars + NUL terminator), exactly like the
/// C `ulidgen_r(char[27])`.
///
/// Reads the previous contents of `ulid` to detect the same-millisecond case
/// and increment the random part in place (with sleep + recursion on
/// Z-carry overflow); otherwise fills the random part via `getentropy`.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    let alphabet = B32_ALPHABET.as_bytes();

    // Get current time in milliseconds
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch");
    let mut t = now.as_secs() as u64 * 1000 + now.subsec_nanos() as u64 / 1_000_000;

    // Encode the 48-bit timestamp into the first 10 chars
    let mut same = true;
    for i in (0..10).rev() {
        let c = alphabet[(t % 32) as usize];
        if ulid[i] != c {
            same = false;
        }
        ulid[i] = c;
        t /= 32;
    }

    if same {
        // Same millisecond: increment the random part in place
        let mut all_z = true;
        let mut invalid = false;
        for j in (0..16).rev() {
            let pos = 10 + j;
            if ulid[pos] == b'Z' {
                ulid[pos] = b'0';
            } else if alphabet.contains(&ulid[pos]) {
                let idx = alphabet.iter().position(|&c| c == ulid[pos]).unwrap();
                ulid[pos] = alphabet[idx + 1];
                all_z = false;
                break;
            } else {
                invalid = true;
                break;
            }
        }

        if invalid {
            // Invalid character: fall through to full re-randomization
            let mut rnd = [0u8; 16];
            if std::os::unix::fs::getentropy(&mut rnd).is_err() {
                std::process::abort();
            }
            for i in 0..16 {
                ulid[10 + i] = alphabet[rnd[i] as usize % 32];
            }
            ulid[26] = 0;
            return;
        }

        if all_z {
            // Carry overflow: sleep and recurse
            std::thread::sleep(std::time::Duration::from_nanos(1_234_567));
            ulidgen_r(ulid);
            return;
        }
    } else {
        // Fill random part with entropy
        let mut rnd = [0u8; 16];
        if std::os::unix::fs::getentropy(&mut rnd).is_err() {
            std::process::abort();
        }
        for i in 0..16 {
            ulid[10 + i] = alphabet[rnd[i] as usize % 32];
        }
    }

    ulid[26] = 0;
}

/// Ergonomic wrapper: generate one ULID and return it as a 26-char String.
///
/// Generates into a zeroed `[0u8; 27]` buffer via [`ulidgen_r`] and returns
/// the first 26 bytes as a String.
pub fn ulid() -> String {
    let mut buf = [0u8; 27];
    ulidgen_r(&mut buf);
    String::from_utf8_lossy(&buf[..26]).into_owned()
}
