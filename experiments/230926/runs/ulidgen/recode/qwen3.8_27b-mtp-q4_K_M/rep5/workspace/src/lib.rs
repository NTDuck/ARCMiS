//! ulidgen — generate ULID (Universally Unique Lexicographically Sortable Identifier).
//!
//! Port of `src/ulid.c` (public domain, Leah Neukirchen).
//!
//! NOTE (fidelity): `ulidgen_r` is stateful in the *caller's* buffer — the
//! same-millisecond detection compares against the previous contents of the
//! same buffer, exactly like the C original. Callers must reuse one buffer
//! (as `main` does) to get the increment-on-same-millisecond behavior.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Crockford Base32 alphabet (32 chars, no I/L/O/U).
pub const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into `ulid` (26 chars + NUL at index 26), like C `ulidgen_r`.
///
/// Mirrors `src/ulid.c`:
/// - 10-char big-endian base32 millisecond timestamp in `ulid[0..10)`
/// - 16-char random part in `ulid[10..26)`
/// - same-millisecond: increment random part in place (carry from index 15);
///   on carry-off sleep 1,234,567 ns and recurse
/// - fresh random part: 16 bytes from `getrandom`, mapped `B32_ALPHABET[b % 32]`
///   (modulo bias intentionally preserved for parity with the C tool)
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    let mut same = true;
    ulid[26] = 0;

    // Milliseconds since the Unix epoch (clock_gettime(CLOCK_REALTIME)).
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let mut t = duration.as_secs() * 1000 + duration.subsec_nanos() as u64 / 1_000_000;

    // Encode the timestamp big-endian into 10 base32 digits.
    for i in (0..10).rev() {
        let d = B32_ALPHABET[(t % 32) as usize];
        if ulid[i] != d {
            ulid[i] = d;
            same = false;
        }
        t /= 32;
    }

    if same {
        // Same millisecond as the previous call: increment the random part
        // in place, carrying from the last digit (index 15) backwards.
        let mut i: i32 = 15;
        while i < 16 && ulid[10 + i as usize] == b'Z' {
            ulid[10 + i as usize] = b'0';
            i -= 1;
        }
        if i < 0 {
            // Carry fell off: wait ~1.23 ms and retry.
            std::thread::sleep(Duration::from_nanos(1_234_567));
            ulidgen_r(ulid);
            return;
        }
        // If the digit is a valid alphabet char, advance it to the next one.
        if let Some(pos) = B32_ALPHABET.iter().position(|&c| c == ulid[10 + i as usize]) {
            ulid[10 + i as usize] = B32_ALPHABET[pos + 1];
            return;
        }
        // Otherwise the buffer is corrupted: fall through to re-randomization.
    }

    // Fresh random part: 16 bytes, each mapped through the alphabet.
    let mut rnd = [0u8; 16];
    if getrandom::getrandom(&mut rnd).is_err() {
        std::process::abort();
    }
    for (i, b) in rnd.iter().enumerate() {
        ulid[10 + i as usize] = B32_ALPHABET[(*b % 32) as usize];
    }
}
