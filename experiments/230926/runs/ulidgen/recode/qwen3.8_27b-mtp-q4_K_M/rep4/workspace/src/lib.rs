//! ulidgen — generate ULIDs (Universally Unique Lexicographically Sortable
//! Identifiers).
//!
//! Public-domain port of Leah Neukirchen's C `ulidgen` (see plan.md).
//! Mirrors `src/ulid.c` + `src/ulid.h`.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Crockford Base32 alphabet (no I L O U).
pub const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into `ulid`.
///
/// `ulid` must hold 27 bytes; on success the first 26 bytes are the ULID and
/// `ulid[26]` is set to 0 (NUL), mirroring the C `char[27]` contract so the
/// function can be called repeatedly on the same buffer (same-ms increment).
///
/// Faithful port of C `ulidgen_r(char[27])`:
/// - first 10 chars: ms-since-epoch timestamp, 5-bit groups, least
///   significant group at index 9;
/// - if the millisecond is unchanged (detected by comparing against the
///   buffer's prior contents), the 16-char random part is incremented in
///   place (Z wraps to 0; all-Z sleeps ~1.23 ms and retries via a loop
///   instead of C's recursion; invalid chars trigger re-randomization);
/// - otherwise the random part is filled from 16 `getrandom` bytes.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    // C: ulid[26] = 0;
    ulid[26] = 0;

    // C: clock_gettime(CLOCK_REALTIME) -> t = tv.tv_sec*1000 + tv.tv_nsec/1000000
    let mut t: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_millis() as u64;

    // C: for (i = 9; i >= 0; i--, t /= 32) ulid[i] = b32alphabet[t % 32];
    // `same` is 0 if any char differs from the buffer's prior content.
    let mut same = true;
    for i in (0..10).rev() {
        let c = B32_ALPHABET[(t % 32) as usize];
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        t /= 32;
    }

    if same {
        // Same millisecond as the previous call in this buffer: increment the
        // 16-char random part in place. C recurses on all-Z overflow; we use a
        // loop + sleep retry instead to avoid unbounded stack growth.
        loop {
            let mut all_z = true;
            let mut invalid = false;
            for i in (0..16).rev() {
                let idx = 10 + i;
                let c = ulid[idx];
                if c == b'Z' {
                    ulid[idx] = b'0';
                    continue;
                }
                all_z = false;
                match B32_ALPHABET.iter().position(|&a| a == c) {
                    Some(pos) => {
                        // Advance to the alphabet successor.
                        ulid[idx] = B32_ALPHABET[pos + 1];
                        return;
                    }
                    None => {
                        // Corrupt buffer: fall through to re-randomization.
                        invalid = true;
                        break;
                    }
                }
            }
            if invalid {
                break;
            }
            if all_z {
                // C: nanosleep(0, 1234567); ulidgen_r(ulid);
                std::thread::sleep(Duration::from_nanos(1_234_567));
                continue;
            }
            // Should not reach here (either returned or broke), but be safe.
            return;
        }
    }

    // New millisecond / fresh buffer / corrupt random part: re-randomize.
    // C: getentropy(rnd, 16); abort() on failure.
    let mut rnd = [0u8; 16];
    getrandom::fill(&mut rnd).expect("getentropy failed");
    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize];
    }
}
