//! ulidgen — generate ULIDs (Universally Unique Lexicographically Sortable
//! Identifiers). Rust port of the public-domain C `ulidgen` by Leah Neukirchen.
//!
//! To the extent possible under law, the creator of this work has waived all
//! copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Crockford base32 alphabet (no I, L, O, U).
pub const B32_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Fill `buf` (26 chars + NUL) with a ULID, mirroring C `ulidgen_r(char[27])`.
///
/// The caller MUST reuse the same buffer across calls so the
/// same-millisecond increment logic works exactly as in the C original.
pub fn ulidgen_r(buf: &mut [u8; 27]) {
    let b32 = B32_ALPHABET.as_bytes();
    let mut same = true;

    buf[26] = 0;

    // Timestamp in milliseconds (48-bit value), like clock_gettime(CLOCK_REALTIME).
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let mut t = (d.as_secs() * 1000 + d.as_nanos() as u64 / 1_000_000) as u64;

    // Encode big-endian into buf[0..10]; track `same` (all 10 chars already
    // matched the previous buffer contents).
    for i in (0..10).rev() {
        let c = b32[(t % 32) as usize];
        if buf[i] != c {
            buf[i] = c;
            same = false;
        }
        t /= 32;
    }

    if same {
        // Increment the random part (buf[10..26]) in place.
        let mut i: i32 = 15;
        while i >= 0 && buf[10 + i as usize] == b'Z' {
            buf[10 + i as usize] = b'0';
            i -= 1;
        }

        if i < 0 {
            // Restart ~1 ms + a bit later.
            thread::sleep(Duration::from_nanos(1_234_567));
            ulidgen_r(buf);
            return;
        }

        // Advance buf[10+i] to its successor in the alphabet.
        let idx = 10 + i as usize;
        if let Some(pos) = b32.iter().position(|&c| c == buf[idx]) {
            buf[idx] = b32[pos + 1];
            return;
        }
        // else randomize again when we found invalid chars
    }

    // Random path: 16 bytes for easier encoding (preserve the modulo bias).
    let mut rnd = [0u8; 16];
    if getrandom::getrandom(&mut rnd).is_err() {
        std::process::abort();
    }
    for i in 0..16 {
        buf[10 + i] = b32[(rnd[i] % 32) as usize];
    }
}

/// Convenience: return a fresh ULID as a `String` (26 chars).
///
/// Note: uses a fresh zeroed buffer, so the same-millisecond increment path
/// does NOT apply (unlike the C tests, which reuse one buffer).
pub fn ulid() -> String {
    let mut b = [0u8; 27];
    ulidgen_r(&mut b);
    String::from_utf8(b[..26].to_vec()).unwrap()
}
