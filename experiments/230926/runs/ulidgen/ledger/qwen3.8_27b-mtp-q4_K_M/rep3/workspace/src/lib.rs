/*
 * ulidgen_r - generate ULID
 * (Universally Unique Lexicographically Sortable Identifier)
 *
 * To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
 * has waived all copyright and related or neighboring rights to this work.
 * http://creativecommons.org/publicdomain/zero/1.0/
 */

use std::fs::File;
use std::io::Read;
use std::process;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Crockford base32 alphabet (no I, L, O, U).
const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into `ulid`, mirroring the C `ulidgen_r(char[27])`.
///
/// The buffer holds the previous ULID (26 ASCII chars + NUL at index 26),
/// exactly like the C caller reusing one buffer. If the millisecond
/// timestamp is unchanged, the 16-char random part is incremented in place;
/// otherwise (or on invalid chars) it is re-randomized from /dev/urandom.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    let mut same = true;

    ulid[26] = 0;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch");
    let mut t: u64 = now.as_secs() * 1000 + now.subsec_millis() as u64;

    for i in (0..10).rev() {
        let c = B32_ALPHABET[(t % 32) as usize];
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        t /= 32;
    }

    if same {
        /* increment random part in place */
        let mut i: i64 = 15;
        while i >= 0 && ulid[10 + i as usize] == b'Z' {
            ulid[10 + i as usize] = b'0';
            i -= 1;
        }

        if i < 0 {
            /* restart 1ms + a bit later */
            thread::sleep(Duration::from_nanos(1_234_567));
            ulidgen_r(ulid);
            return;
        }

        if let Some(pos) = B32_ALPHABET.iter().position(|&c| c == ulid[10 + i as usize]) {
            ulid[10 + i as usize] = B32_ALPHABET[pos + 1];
            return;
        }
        /* else randomize again when we found invalid chars */
    }

    let mut rnd = [0u8; 16]; /* use 16 bytes for easier encoding */
    let mut file = match File::open("/dev/urandom") {
        Ok(f) => f,
        Err(_) => process::abort(),
    };
    if file.read_exact(&mut rnd).is_err() {
        process::abort();
    }

    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize];
    }
}
