//! ulidgen_r - generate ULID
//! (Universally Unique Lexicographically Sortable Identifier)
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::io::Read;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into `ulid` (a 27-byte buffer, NUL-terminated).
///
/// If the timestamp part is unchanged from the previous call (same
/// millisecond), the random part is incremented in place. Otherwise the
/// random part is re-randomized from fresh entropy.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    let mut same = true;

    ulid[26] = 0;

    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_millis() as u64;

    let mut tt = t;
    for i in (0..10).rev() {
        let c = B32_ALPHABET[(tt % 32) as usize];
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        tt /= 32;
    }

    if same {
        /* increment random part in place */
        let buf = &mut ulid[10..26];
        let mut i: isize = 15;
        while i >= 0 && buf[i as usize] == b'Z' {
            buf[i as usize] = b'0';
            i -= 1;
        }

        if i < 0 {
            /* restart 1ms + a bit later */
            std::thread::sleep(Duration::from_nanos(1234567));
            ulidgen_r(ulid);
            return;
        }

        let pos = B32_ALPHABET.iter().position(|&c| c == buf[i as usize]);
        if let Some(p) = pos {
            buf[i as usize] = B32_ALPHABET[p + 1];
            return;
        }
        /* else randomize again when we found invalid chars */
    }

    let mut rnd = [0u8; 16]; /* use 16 bytes for easier encoding */
    let mut f = std::fs::File::open("/dev/urandom").expect("open /dev/urandom");
    if f.read_exact(&mut rnd).is_err() {
        std::process::abort();
    }

    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize];
    }
}
