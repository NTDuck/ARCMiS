//! ulidgen - generate ULID
//! (Universally Unique Lexicographically Sortable Identifier)
//!
//! Rust translation of Leah Neukirchen's `ulid.c`.
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to the
//! original C work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::io::Read;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Crockford base32 alphabet (no I, L, O, U).
const B32: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// A ULID generator.
///
/// The C original relies on the caller reusing the same 27-byte output
/// buffer between calls: `ulidgen_r` compares the new timestamp against the
/// *previous* call's timestamp to decide whether to increment the random
/// part in place. We model that explicitly: the generator keeps the last
/// ULID it produced (26 chars) as its state.
pub struct UlidGen {
    /// Last ULID produced (26 chars), or all NULs before the first call.
    last: [u8; 26],
}

impl UlidGen {
    /// Create a fresh generator (no previous ULID, like a zeroed buffer).
    pub fn new() -> Self {
        UlidGen { last: [0u8; 26] }
    }

    /// Generate the next ULID, mirroring `ulidgen_r` exactly:
    ///
    /// 1. Compute the current time in milliseconds since the epoch.
    /// 2. Encode it as 10 Crockford base32 chars (most significant first,
    ///    `t /= 32` each step, exactly like the C loop).
    /// 3. If all 10 timestamp chars equal the previous call's, increment
    ///    the 16-char random part in place (trailing 'Z's roll over to
    ///    '0'); if it overflows entirely, sleep 1234567 ns and retry;
    ///    if the char to advance is not in the alphabet, re-randomize.
    /// 4. Otherwise fill the random part from 16 fresh random bytes.
    pub fn next(&mut self) -> String {
        loop {
            let mut ulid = self.last;

            let mut t = now_ms();
            let mut same = true;
            for i in (0..10).rev() {
                let c = B32[(t % 32) as usize];
                if ulid[i] != c {
                    ulid[i] = c;
                    same = false;
                }
                t /= 32;
            }

            if same {
                // increment random part in place
                let mut i: i32 = 15;
                while i >= 0 && ulid[10 + i as usize] == b'Z' {
                    ulid[10 + i as usize] = b'0';
                    i -= 1;
                }

                if i < 0 {
                    // restart 1ms + a bit later
                    std::thread::sleep(Duration::from_nanos(1234567));
                    continue; // retry (C recurses with the same buffer)
                }

                if let Some(pos) = B32.iter().position(|&c| c == ulid[10 + i as usize]) {
                    ulid[10 + i as usize] = B32[pos + 1];
                    self.last = ulid;
                    return String::from_utf8(ulid.to_vec()).expect("ULID is ASCII");
                }
                // else: found invalid chars, fall through and randomize again
            }

            // randomize the 16-char random part
            let rnd = random16();
            for i in 0..16 {
                ulid[10 + i] = B32[rnd[i] as usize % 32];
            }
            self.last = ulid;
            return String::from_utf8(ulid.to_vec()).expect("ULID is ASCII");
        }
    }
}

impl Default for UlidGen {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience: generate a ULID from a process-wide default generator,
/// so callers (tests, CLI) can just ask for the next ULID.
pub fn next_ulid() -> String {
    static GEN: Mutex<UlidGen> = Mutex::new(UlidGen { last: [0u8; 26] });
    GEN.lock().expect("ulidgen lock poisoned").next()
}

/// Current time in milliseconds since the Unix epoch.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_millis() as u64
}

/// Read 16 random bytes from /dev/urandom (equivalent to getentropy(16)).
/// Aborts on failure, like the C code.
fn random16() -> [u8; 16] {
    let mut f = match std::fs::File::open("/dev/urandom") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("ulidgen: cannot open /dev/urandom: {e}");
            std::process::abort();
        }
    };
    let mut buf = [0u8; 16];
    if f.read_exact(&mut buf).is_err() {
        eprintln!("ulidgen: cannot read /dev/urandom");
        std::process::abort();
    }
    buf
}
