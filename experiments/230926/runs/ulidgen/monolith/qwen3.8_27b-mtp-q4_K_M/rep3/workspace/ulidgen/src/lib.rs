//! ulidgen_r - generate ULID
//! (Universally Unique Lexicographically Sortable Identifier)
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::io::Read;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const B32_ALPHABET: [u8; 32] = *b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID, mirroring the C `ulidgen_r(char ulid[27])`.
///
/// `prev` is the previously generated ULID (or an empty string on the first
/// call).  It is used to detect same-millisecond collisions: if the
/// timestamp part did not change, the random part is incremented in place
/// instead of being re-randomized.
pub fn ulidgen_r(prev: &str) -> String {
    let mut ulid: Vec<u8> = if prev.len() == 26 {
        prev.as_bytes().to_vec()
    } else {
        vec![b'0'; 26]
    };
    let mut same = true;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch");
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
        // increment random part in place
        let mut i: isize = 15;
        while i >= 0 && ulid[10 + i as usize] == b'Z' {
            ulid[10 + i as usize] = b'0';
            i -= 1;
        }

        if i < 0 {
            // restart 1ms + a bit later
            std::thread::sleep(Duration::from_nanos(1234567));
            return ulidgen_r(&String::from_utf8(ulid).expect("ascii"));
        }

        let pos = B32_ALPHABET.iter().position(|&c| c == ulid[10 + i as usize]);
        if let Some(pos) = pos {
            ulid[10 + i as usize] = B32_ALPHABET[pos + 1];
            return String::from_utf8(ulid).expect("ascii");
        }
        // else randomize again when we found invalid chars
    }

    let mut rnd = [0u8; 16]; // use 16 bytes for easier encoding
    get_entropy(&mut rnd);

    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize];
    }

    String::from_utf8(ulid).expect("ascii")
}

/// Fill `buf` with random bytes, aborting on failure (like getentropy).
fn get_entropy(buf: &mut [u8]) {
    let mut f = match std::fs::File::open("/dev/urandom") {
        Ok(f) => f,
        Err(_) => std::process::abort(),
    };
    if f.read_exact(buf).is_err() {
        std::process::abort();
    }
}
