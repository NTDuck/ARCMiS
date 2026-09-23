/*
 * ulidgen_r - generate ULID
 * (Universally Unique Lexicographically Sortable Identifier)
 *
 * To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
 * has waived all copyright and related or neighboring rights to this work.
 * http://creativecommons.org/publicdomain/zero/1.0/
 */

/// Generate a ULID into `ulid`, a 27-byte buffer (26 characters + NUL).
///
/// This is the Rust translation of the C `ulidgen_r(char ulid[27])`:
/// the first 10 characters encode the current time in milliseconds,
/// the remaining 16 characters are random (or incremented in place
/// when the timestamp did not change).
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    const B32: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    let mut same = true;

    ulid[26] = 0;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before the UNIX epoch");
    let mut t: u64 = now.as_secs() * 1000 + now.subsec_nanos() as u64 / 1_000_000;

    for i in (0..10).rev() {
        t /= 32;
        let c = B32[(t % 32) as usize];
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
    }

    if same {
        // increment random part in place (buf[i] == ulid[10 + i])
        let mut i: isize = 15;
        while i >= 0 && ulid[10 + i as usize] == b'Z' {
            ulid[10 + i as usize] = b'0';
            i -= 1;
        }

        if i < 0 {
            // restart 1ms + a bit later
            std::thread::sleep(std::time::Duration::from_nanos(1234567));
            ulidgen_r(ulid);
            return;
        }

        let c = ulid[10 + i as usize];
        if let Some(pos) = B32.iter().position(|&x| x == c) {
            ulid[10 + i as usize] = B32[pos + 1];
            return;
        }
        // else randomize again when we found invalid chars
    }

    let mut rnd = [0u8; 16]; // use 16 bytes for easier encoding
    if !read_entropy(&mut rnd) {
        std::process::abort();
    }

    for i in 0..16 {
        ulid[10 + i] = B32[(rnd[i] % 32) as usize];
    }
}

/// Fill `buf` with random bytes from the kernel entropy source.
/// Returns `false` on failure (the caller aborts, like the C original).
fn read_entropy(buf: &mut [u8]) -> bool {
    use std::io::Read;
    match std::fs::File::open("/dev/urandom") {
        Ok(mut f) => f.read_exact(buf).is_ok(),
        Err(_) => false,
    }
}
