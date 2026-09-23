/*
 * ulidgen_r - generate ULID
 * (Universally Unique Lexicographically Sortable Identifier)
 *
 * To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
 * has waived all copyright and related or neighboring rights to this work.
 * http://creativecommons.org/publicdomain/zero/1.0/
 */

use std::io::Read;

const B32_ALPHABET: [u8; 32] = *b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into `ulid` (27 bytes, NUL-terminated like the C version).
///
/// If the timestamp part is unchanged from the previous call, the random
/// part is incremented in place; otherwise it is re-randomized.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    let mut same = true;

    ulid[26] = 0;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let mut t = now.as_secs() * 1000 + now.subsec_nanos() as u64 / 1_000_000;

    for i in (0..10).rev() {
        let c = B32_ALPHABET[(t % 32) as usize];
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        t /= 32;
    }

    if same {
        /* increment random part in place (buf = ulid + 10) */
        let mut i: isize = 15;
        while i >= 0 && ulid[10 + i as usize] == b'Z' {
            ulid[10 + i as usize] = b'0';
            i -= 1;
        }

        if i < 0 {
            /* restart 1ms + a bit later */
            std::thread::sleep(std::time::Duration::from_nanos(1234567));
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
    let mut f = match std::fs::File::open("/dev/urandom") {
        Ok(f) => f,
        Err(_) => std::process::abort(),
    };
    if f.read_exact(&mut rnd).is_err() {
        std::process::abort();
    }

    for i in 0..16 {
        ulid[10 + i] = B32_ALPHABET[(rnd[i] % 32) as usize];
    }
}
