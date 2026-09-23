//! Integration tests mirroring `tests/test.c`.

use std::time::Duration;

const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a ULID into a fresh buffer (no previous ULID).
fn fresh_ulid() -> [u8; 26] {
    let mut buf = [b'0'; 27];
    ulidgen::ulidgen_r(&mut buf);
    buf[..26].try_into().unwrap()
}

/// test_ulid_length: a generated ULID is 26 chars.
#[test]
fn test_ulid_length() {
    let mut buf = [b'0'; 27];
    ulidgen::ulidgen_r(&mut buf);
    let s = std::str::from_utf8(&buf[..26]).unwrap();
    assert_eq!(s.len(), 26);
    assert_eq!(buf[26], 0, "NUL terminator kept at index 26");
}

/// test_ulid_structure: every char is in the Crockford base32 alphabet.
#[test]
fn test_ulid_structure() {
    let ulid = fresh_ulid();
    for c in ulid.iter() {
        assert!(
            ALPHABET.contains(c),
            "invalid char {c:?} not in Crockford base32 alphabet"
        );
    }
}

/// test_ulid_uniqueness: two consecutive ULIDs (same buffer) differ.
#[test]
fn test_ulid_uniqueness() {
    let mut buf = [b'0'; 27];
    ulidgen::ulidgen_r(&mut buf);
    let first: [u8; 26] = buf[..26].try_into().unwrap();
    ulidgen::ulidgen_r(&mut buf);
    let second: [u8; 26] = buf[..26].try_into().unwrap();
    assert_ne!(first, second, "consecutive ULIDs must differ");
}

/// test_ulid_sortability: a ULID generated after a 1.5 ms delay sorts
/// lexicographically after the first one.
#[test]
fn test_ulid_sortability() {
    let mut buf = [b'0'; 27];
    ulidgen::ulidgen_r(&mut buf);
    let first: [u8; 26] = buf[..26].try_into().unwrap();
    std::thread::sleep(Duration::from_millis(1) + Duration::from_micros(500));
    ulidgen::ulidgen_r(&mut buf);
    let second: [u8; 26] = buf[..26].try_into().unwrap();
    assert!(
        first < second,
        "ULIDs must sort lexicographically: {first:?} !< {second:?}"
    );
}
