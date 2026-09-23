//! ulidgen_r — generate ULIDs (Universally Unique Lexicographically
//! Sortable Identifiers).
//!
//! Rust port of `src/ulid.c` (public domain, Leah Neukirchen).
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::process;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Crockford-style base32 alphabet (no I, L, O, U).
const B32_ALPHABET: [u8; 32] = *b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into `ulid` (26 visible chars + NUL at index 26).
///
/// Faithful port of the C `ulidgen_r(char ulid[27])`:
/// the buffer is both input and output — if it already holds a ULID from
/// the *current* millisecond, the 16-char random part is incremented in
/// place instead of being re-randomized.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    let mut same = true;

    ulid[26] = 0;

    // clock_gettime(CLOCK_REALTIME) -> ms since epoch
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX epoch");
    let mut t: u64 = now.as_secs() * 1000 + now.subsec_nanos() as u64 / 1_000_000;

    // for (i = 9; i >= 0; i--, t /= 32) ulid[i] = alphabet[t % 32]
    for i in (0..10).rev() {
        let d = (t % 32) as usize;
        if ulid[i] != B32_ALPHABET[d] {
            ulid[i] = B32_ALPHABET[d];
            same = false;
        }
        t /= 32;
    }

    let buf = &mut ulid[10..];

    if same {
        // increment random part in place
        let mut i: isize = 15;
        while i >= 0 && buf[i as usize] == b'Z' {
            buf[i as usize] = b'0';
            i -= 1;
        }

        if i < 0 {
            // restart 1ms + a bit later
            thread::sleep(Duration::from_nanos(1_234_567));
            ulidgen_r(ulid);
            return;
        }

        // strchr(b32alphabet, buf[i]); if found, advance to next char
        if let Some(pos) = B32_ALPHABET.iter().position(|&c| c == buf[i as usize]) {
            buf[i as usize] = B32_ALPHABET[pos + 1];
            return;
        }
        // else randomize again when we found invalid chars
    }

    // getentropy(rnd, 16); abort() on failure
    let mut rnd = [0u8; 16]; // use 16 bytes for easier encoding
    if getrandom::getrandom(&mut rnd).is_err() {
        process::abort();
    }

    for i in 0..16 {
        buf[i] = B32_ALPHABET[(rnd[i] % 32) as usize];
    }
}
