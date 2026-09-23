/*
 * ulidgen_r - generate ULID
 * (Universally Unique Lexicographically Sortable Identifier)
 *
 * To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
 * has waived all copyright and related or neighboring rights to this work.
 * http://creativecommons.org/publicdomain/zero/1.0/
 */

/// Crockford Base32 alphabet.
const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into `ulid` (27 bytes, NUL-terminated like the C API).
///
/// If the timestamp part is unchanged from the previous call, the random
/// part is incremented in place; otherwise it is re-randomized.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    let mut same = true;

    ulid[26] = 0;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before the UNIX epoch");
    let mut t: u64 = now.as_secs() * 1000 + now.subsec_nanos() as u64 / 1_000_000;

    for i in (0..10).rev() {
        let c = B32_ALPHABET[(t % 32) as usize];
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        t /= 32;
    }

    let buf: &mut [u8] = &mut ulid[10..26];

    if same {
        /* increment random part in place */
        let mut i: isize = 15;
        while i >= 0 && buf[i as usize] == b'Z' {
            buf[i as usize] = b'0';
            i -= 1;
        }

        if i < 0 {
            /* restart 1ms + a bit later */
            std::thread::sleep(std::time::Duration::from_nanos(1_234_567));
            ulidgen_r(ulid);
            return;
        }

        let i = i as usize;
        if let Some(pos) = B32_ALPHABET.iter().position(|&c| c == buf[i]) {
            if pos + 1 < B32_ALPHABET.len() {
                buf[i] = B32_ALPHABET[pos + 1];
                return;
            }
        }
        /* else randomize again when we found invalid chars */
    }

    let mut rnd = [0u8; 16]; /* use 16 bytes for easier encoding */
    get_entropy(&mut rnd);

    for i in 0..16 {
        buf[i] = B32_ALPHABET[(rnd[i] % 32) as usize];
    }
}

/// Fill `buf` with random bytes, aborting on failure (like getentropy(3)).
fn get_entropy(buf: &mut [u8]) {
    use std::io::Read;
    match std::fs::File::open("/dev/urandom") {
        Ok(mut f) => {
            if f.read_exact(buf).is_err() {
                std::process::abort();
            }
        }
        Err(_) => std::process::abort(),
    }
}
