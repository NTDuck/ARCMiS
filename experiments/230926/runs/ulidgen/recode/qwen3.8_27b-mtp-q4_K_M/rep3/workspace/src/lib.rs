//! Core ULID generation — translation of `src/ulid.c` + `src/ulid.h`.
//!
//! A ULID is 26 Crockford-Base32 chars: 10 timestamp chars + 16 random chars.
//! The generator is STATEFUL (mirrors the C "reused buffer" semantics): when
//! two calls land in the same millisecond, the 16-char random part is
//! incremented in place instead of re-randomized.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use getrandom::getrandom;

/// Crockford-Base32 alphabet (no I/L/O/U). (C: `b32alphabet`)
pub const B32: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Stateful ULID generator. (C: `ulidgen_r(char ulid[27])`)
///
/// `last` holds the previously generated ULID and mirrors the C buffer-reuse
/// state that the `same`-millisecond detection depends on.
#[derive(Default)]
pub struct UlidGen {
    last: [u8; 26],
}

impl UlidGen {
    /// Create a fresh generator (C: zeroed buffer).
    pub fn new() -> Self {
        Self { last: [0u8; 26] }
    }

    /// Generate the next ULID and return it as a 26-char string.
    ///
    /// Faithful port of `ulidgen_r`:
    /// 1. Start from `self.last` (C buffer reuse); `same = true`.
    /// 2. `t = SystemTime::now().duration_since(UNIX_EPOCH).as_millis()`.
    /// 3. Encode `t` big-endian into chars 0..10 via `B32[(t % 32)]`,
    ///    clearing `same` on any change.
    /// 4. If `same`: increment chars 10..26 right-to-left (zero trailing
    ///    `Z`s, bump first non-`Z` to its successor in `B32`, guard
    ///    `pos + 1 < 32`); on all-`Z` overflow sleep 1_234_567 ns and retry
    ///    (C: `nanosleep` + recursion); on invalid char fall through.
    /// 5. Otherwise fill chars 10..26 from 16 `getrandom` bytes:
    ///    `B32[(rnd[i] % 32)]`; `abort()` on entropy failure.
    /// 6. Store into `self.last` and return the string.
    pub fn next(&mut self) -> String {
        let mut ulid = self.last; // start from previous value (C buffer reuse)
        let mut same = true;

        let mut t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Encode the timestamp into ulid[0..10], most-significant digit first.
        for i in (0..10).rev() {
            let c = B32[(t % 32) as usize];
            if ulid[i] != c {
                ulid[i] = c;
                same = false;
            }
            t /= 32;
        }

        if same {
            // Same millisecond as the previous call: increment the 16-char
            // random region (ulid[10..26]) right-to-left.
            let mut i = 25usize;
            while i >= 10 && ulid[i] == b'Z' {
                ulid[i] = b'0';
                i -= 1;
            }
            if i < 10 {
                // All Z: wait ~1.23 ms and retry (C: nanosleep + recursion).
                std::thread::sleep(Duration::from_nanos(1_234_567));
                return self.next();
            }
            if let Some(pos) = B32.iter().position(|&c| c == ulid[i]) {
                if pos + 1 < 32 {
                    ulid[i] = B32[pos + 1];
                    self.last = ulid;
                    return String::from_utf8(ulid.to_vec()).unwrap();
                }
                // else: invalid/edge char -> fall through to re-randomize.
            }
        }

        // New millisecond: randomize the 16 random bytes (C: getentropy).
        let mut rnd = [0u8; 16];
        if getrandom(&mut rnd).is_err() {
            std::process::abort();
        }
        for i in 0..16 {
            ulid[10 + i] = B32[(rnd[i] % 32) as usize];
        }

        self.last = ulid;
        String::from_utf8(ulid.to_vec()).unwrap()
    }
}
