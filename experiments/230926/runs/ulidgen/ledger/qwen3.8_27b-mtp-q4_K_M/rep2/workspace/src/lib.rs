//! ulidgen_r - generate ULID
//! (Universally Unique Lexicographically Sortable Identifier)
//!
//! Faithful port of `src/ulid.c`. The caller owns the 27-byte buffer
//! (26 chars + NUL terminator); the previous ULID left in the buffer is
//! the implicit state that drives the `same` flag.

use std::fs::OpenOptions;
use std::io::Read;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Generate a ULID into the caller-owned 27-byte buffer.
///
/// `ulid[0..26]` holds the 26 base32 characters, `ulid[26]` is set to NUL.
/// If the timestamp part is unchanged from the previous call (the `same`
/// flag), the random part `ulid[10..26]` is incremented in place; otherwise
/// it is re-randomized from `/dev/urandom`.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    const B32ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    let mut same = true;

    ulid[26] = 0;

    // Milliseconds since the Unix epoch (truncated, like the C version).
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // Encode 10 base-32 digits big-endian into ulid[0..10].
    let mut t = t;
    for i in (0..10).rev() {
        let c = B32ALPHABET[(t % 32) as usize];
        if ulid[i] != c {
            ulid[i] = c;
            same = false;
        }
        t /= 32;
    }

    if same {
        // Increment random part in place.
        let mut i: usize = 15;
        while i <= 15 && ulid[10 + i] == b'Z' {
            ulid[10 + i] = b'0';
            if i == 0 {
                i = usize::MAX; // signals underflow (C: i < 0)
                break;
            }
            i -= 1;
        }

        if i == usize::MAX {
            // All 16 chars were 'Z': restart 1ms + a bit later.
            std::thread::sleep(Duration::from_nanos(1_234_567));
            ulidgen_r(ulid);
            return;
        }

        let c = ulid[10 + i];
        if let Some(p) = B32ALPHABET.iter().position(|&a| a == c) {
            if p + 1 < B32ALPHABET.len() {
                ulid[10 + i] = B32ALPHABET[p + 1];
                return;
            }
        }
        // else randomize again when we found invalid chars
    }

    // Re-randomize: 16 bytes for easier encoding.
    let mut rnd = [0u8; 16];
    let mut file = OpenOptions::new()
        .read(true)
        .open("/dev/urandom")
        .unwrap_or_else(|_| std::process::abort());
    if file.read_exact(&mut rnd).is_err() {
        std::process::abort();
    }

    for i in 0..16 {
        ulid[10 + i] = B32ALPHABET[(rnd[i] % 32) as usize];
    }
}
