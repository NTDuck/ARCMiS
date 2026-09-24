//! ulidgen — generate ULIDs (Universally Unique Lexicographically Sortable
//! Identifiers).
//!
//! Port of the public-domain C `ulidgen_r` (src/ulid.c).
//!
//! To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
//! has waived all copyright and related or neighboring rights to this work.
//! http://creativecommons.org/publicdomain/zero/1.0/

use std::time::Duration;

/// Crockford Base32 alphabet (32 chars, no I, L, O, U).
/// Port of the static `b32alphabet` in src/ulid.c.
pub const B32_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Port of C `ulidgen_r(char ulid[27])`.
///
/// `ulid` must hold the previously generated ULID (26 bytes) or be zeroed;
/// on return it holds a new 26-byte ULID. The 27th byte (`ulid[26]`) is a
/// sentinel mirroring the C NUL terminator and stays `0`.
///
/// Behavior mirrors the C source 1:1:
/// - encode the current millisecond timestamp into `ulid[0..10]`;
/// - if the timestamp part is unchanged (same millisecond, caller reused the
///   buffer), increment the 16-char random part in place; if it wraps from
///   all-'Z', sleep ~1.23 ms and recurse;
/// - otherwise fill `ulid[10..26]` from `getrandom::fill` (abort on failure).
pub fn ulidgen_r(ulid: &mut [u8; 27]) {
    // TODO: implement (see design.md section 3, src/lib.rs)
    let _ = (ulid, B32_ALPHABET, Duration::from_nanos(1_234_567));
    todo!("ulidgen_r not yet implemented")
}

/// Convenience: generate a fresh ULID as a `String` (no prior state).
/// Starts from a zeroed buffer, so the first call always randomizes —
/// same as C's zero-initialized `char ulid[27] = { 0 }`.
pub fn ulid() -> String {
    // TODO: implement
    let mut buf = [0u8; 27];
    ulidgen_r(&mut buf);
    String::from_utf8(buf[..26].to_vec()).expect("ULID is always valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alphabet_is_32_chars() {
        // TODO: implement
        assert_eq!(B32_ALPHABET.len(), 32);
    }

    #[test]
    fn ulid_length() {
        // TODO: implement
        let u = ulid();
        assert_eq!(u.len(), 26);
    }

    #[test]
    fn ulid_structure() {
        // TODO: implement
        let u = ulid();
        assert!(u.bytes().all(|b| B32_ALPHABET.contains(&b)));
    }

    #[test]
    fn ulid_uniqueness() {
        // TODO: implement
        assert_ne!(ulid(), ulid());
    }

    #[test]
    fn ulid_sortability() {
        // TODO: implement
        let a = ulid();
        std::thread::sleep(Duration::from_millis(2));
        let b = ulid();
        assert!(a < b);
    }
}
