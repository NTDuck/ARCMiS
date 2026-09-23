//! Rust translation of the C `ulidgen` ULID generator.
//!
//! # API choice
//!
//! The C function `ulidgen_r(char ulid[27])` is *reusable*: the caller keeps
//! the same 27-byte buffer (26 ULID chars + NUL) across calls, and the
//! function reads the previous ULID from the buffer to decide whether to
//! increment the random part in place (same millisecond) or to randomize it
//! (new millisecond).
//!
//! We mirror that exactly with:
//!
//! ```rust,ignore
//! pub fn ulidgen_r(ulid: &mut [u8; 27])
//! ```
//!
//! - The buffer holds the previous ULID (26 chars) plus a NUL byte at index
//!   26 (kept for fidelity with the C signature; the function always resets
//!   `ulid[26] = 0`).
//! - The function is **stateless** (no global/static state), so it is
//!   thread-safe: each thread owns its own buffer, and concurrent calls with
//!   distinct buffers cannot interfere.
//! - A buffer whose first 26 bytes are not a valid Crockford-base32 ULID is
//!   treated as "no previous ULID" (first call), matching the C behavior of
//!   randomizing when the previous content does not match.
//!
//! # Semantics preserved from the C code
//!
//! - Timestamp: current wall-clock time in milliseconds since the Unix epoch,
//!   encoded as 10 Crockford base32 characters (most significant first,
//!   `t /= 32` ten times).
//! - If the new timestamp encodes to the same 10 characters as the previous
//!   ULID, the 16-character random part is incremented in place with
//!   Crockford base32 carry (`'Z'` wraps to `'0'`).
//! - On full overflow (all 16 random chars were `'Z'`), the function sleeps
//!   ~1.23 ms (`nanosleep(0, 1234567)` in C) and retries.
//! - Otherwise the random part is filled from 16 random bytes, each mapped
//!   via `byte % 32` into the alphabet. The C code uses `getentropy(2)` and
//!   `abort()`s on failure; here we read 16 bytes from `/dev/urandom`
//!   (std-only, zero dependencies) and `panic!` on failure.
//!
//! # Example
//!
//! ```
//! use ulidgen::ulidgen_r;
//! let mut ulid = [b'0'; 27];
//! ulidgen_r(&mut ulid);
//! let s = std::str::from_utf8(&ulid[..26]).unwrap();
//! assert_eq!(s.len(), 26);
//! ```

use std::io::Read;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Crockford base32 alphabet, exactly as in the C source.
const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Current wall-clock time in milliseconds since the Unix epoch.
///
/// Mirrors `clock_gettime(CLOCK_REALTIME)` + `tv.tv_sec*1000 + tv.tv_nsec/1000000`.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_millis() as u64
}

/// Read 16 random bytes from `/dev/urandom`.
///
/// std-only replacement for the C `getentropy(rnd, 16)` call.
/// Panics on failure, mirroring the C `abort()`.
fn random_bytes_16() -> [u8; 16] {
    let mut f = std::fs::File::open("/dev/urandom")
        .expect("failed to open /dev/urandom (getentropy equivalent failed)");
    let mut buf = [0u8; 16];
    f.read_exact(&mut buf)
        .expect("failed to read 16 bytes from /dev/urandom (getentropy equivalent failed)");
    buf
}

/// True if `buf` (26 bytes) is a valid Crockford base32 string.
fn is_valid_ulid(buf: &[u8; 26]) -> bool {
    buf.iter().all(|c| B32_ALPHABET.contains(c))
}

/// Generate a ULID into `ulid`, reusing the previous ULID stored in the
/// buffer (C `ulidgen_r(char ulid[27])`).
///
/// `ulid` must be 27 bytes: 26 ULID characters plus a NUL terminator at
/// index 26 (always reset to 0 by this function). The previous ULID (if
/// valid) is read from the buffer to implement the same-millisecond
/// increment logic; the buffer is overwritten with the new ULID.
///
/// Stateless and thread-safe: no global state is used, so each thread can
/// call this with its own buffer concurrently.
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    ulid[26] = 0;

    loop {
        // Timestamp: current time in ms, encoded as 10 Crockford base32
        // chars, most significant first (C: `for (i = 9; i >= 0; i--, t /= 32)`).
        let mut t = now_millis();
        let mut ts = [0u8; 10];
        for i in (0..10).rev() {
            ts[i] = B32_ALPHABET[(t % 32) as usize];
            t /= 32;
        }

        // Read the previous ULID from the buffer (reusable-buffer semantics),
        // then write the new timestamp chars into the buffer (C writes
        // ulid[0..10] before the same-millisecond check).
        let prev: [u8; 26] = ulid[..26].try_into().unwrap();
        let same = is_valid_ulid(&prev) && prev[..10] == ts;
        ulid[..10].copy_from_slice(&ts);

        if same {
            // Increment the random part in place (Crockford base32 carry,
            // 'Z' wraps to '0'), mirroring the C loop over buf[15..=0].
            let mut i = 15usize;
            let mut overflow = false;
            while i >= 1 && ulid[10 + i] == b'Z' {
                ulid[10 + i] = b'0';
                i -= 1;
            }
            if ulid[10 + i] == b'Z' {
                ulid[10 + i] = b'0';
                overflow = true;
            }
            if overflow {
                // Full carry: restart ~1.23 ms later (C: nanosleep 1234567 ns
                // + recursive ulidgen_r). The timestamp prefix is unchanged,
                // so the next iteration re-checks it.
                std::thread::sleep(Duration::from_nanos(1_234_567));
                continue;
            }
            let c = ulid[10 + i];
            if let Some(pos) = B32_ALPHABET.iter().position(|a| *a == c) {
                // C: `char *s = strchr(b32alphabet, buf[i]); buf[i] = *(s+1);`
                // (pos < 32 here because 'Z' was handled by the carry loop).
                ulid[10 + i] = B32_ALPHABET[pos + 1];
                return;
            }
            // Invalid char found: fall through and randomize again (C does
            // the same).
        }

        // Randomize the 16-char random part from 16 random bytes.
        let rnd = random_bytes_16();
        for i in 0..16 {
            ulid[10 + i] = B32_ALPHABET[rnd[i] as usize % 32];
        }
        return;
    }
}
