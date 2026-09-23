/*
 * ulidgen_r - generate ULID
 * (Universally Unique Lexicographically Sortable Identifier)
 *
 * To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
 * has waived all copyright and related or neighboring rights to this work.
 * http://creativecommons.org/publicdomain/zero/1.0/
 */

/// Generate a ULID (Universally Unique Lexicographically Sortable Identifier).
///
/// The result is written into `ulid`, which must be a 27-byte buffer:
/// 26 characters of the ULID followed by a NUL byte (mirroring the C API).
///
/// If the timestamp part is unchanged from the previous call, the random
/// part is incremented in place so that consecutive ULIDs remain unique and
/// lexicographically sortable.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    let mut same = true;

    ulid[26] = 0;

    let mut t = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        now.as_secs() as u64 * 1000 + now.subsec_millis() as u64
    };

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
        let mut i: i32 = 15;
        {
            let (_, buf) = ulid.split_at_mut(10);
            while i >= 0 && buf[i as usize] == b'Z' {
                buf[i as usize] = b'0';
                i -= 1;
            }
        }

        if i < 0 {
            // restart 1ms + a bit later
            std::thread::sleep(std::time::Duration::from_micros(1234));
            ulidgen_r(ulid);
            return;
        }

        {
            let (_, buf) = ulid.split_at_mut(10);
            if let Some(pos) = B32_ALPHABET.iter().position(|&c| c == buf[i as usize]) {
                if pos + 1 < B32_ALPHABET.len() {
                    buf[i as usize] = B32_ALPHABET[pos + 1];
                    return;
                }
                // else randomize again when we found invalid chars
            }
        }
    }

    let mut rnd = [0u8; 16]; // use 16 bytes for easier encoding
    if read_entropy(&mut rnd).is_err() {
        std::process::abort();
    }

    {
        let (_, buf) = ulid.split_at_mut(10);
        for i in 0..16 {
            buf[i] = B32_ALPHABET[rnd[i] as usize % 32];
        }
    }
}

/// Fill `buf` with random bytes from the system entropy source.
fn read_entropy(buf: &mut [u8]) -> std::io::Result<()> {
    use std::io::Read;
    let mut f = std::fs::File::open("/dev/urandom")?;
    f.read_exact(buf)
}
