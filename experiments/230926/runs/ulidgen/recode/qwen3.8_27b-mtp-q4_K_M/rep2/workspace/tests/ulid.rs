//! Integration tests — port of tests/test.c, run via `cargo test`.

use std::time::Duration;

use ulidgen::{ulid, ulidgen_r, B32_ALPHABET};

/// Port of tests/test.c:is_valid_ulid — length 26 and all chars in the
/// Crockford base32 alphabet.
fn is_valid_ulid(s: &str) -> bool {
    s.len() == 26 && s.bytes().all(|b| B32_ALPHABET.contains(&b))
}

/// Port of tests/test.c:test_ulid_length.
#[test]
fn test_ulid_length() {
    let mut buf = [0u8; 27];
    ulidgen_r(&mut buf);
    assert_eq!(buf[26], 0);
    assert_eq!(ulid().len(), 26);
}

/// Port of tests/test.c:test_ulid_structure.
#[test]
fn test_ulid_structure() {
    assert!(is_valid_ulid(&ulid()));
}

/// Port of tests/test.c:test_ulid_uniqueness.
#[test]
fn test_ulid_uniqueness() {
    assert_ne!(ulid(), ulid());
}

/// Port of tests/test.c:test_ulid_sortability (1.5 ms sleep between calls).
#[test]
fn test_ulid_sortability() {
    let a = ulid();
    std::thread::sleep(Duration::from_micros(1500));
    let b = ulid();
    assert!(a < b);
}
