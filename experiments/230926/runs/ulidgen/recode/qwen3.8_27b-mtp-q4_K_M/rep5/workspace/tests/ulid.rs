//! Integration tests ported from `tests/test.c`, run via `cargo test`.

use std::time::Duration;

use ulidgen::ulidgen;

/// Port of `is_valid_ulid` from `tests/test.c`.
fn is_valid_ulid(ulid: &str) -> bool {
    ulid.len() == 26 && ulid.chars().all(|c| ulidgen::B32_ALPHABET.contains(c))
}

/// Port of `test_ulid_length` from `tests/test.c`.
#[test]
fn test_ulid_length() {
    assert_eq!(ulidgen(None).len(), 26);
}

/// Port of `test_ulid_structure` from `tests/test.c`
/// (disabled in the C main; enabled here).
#[test]
fn test_ulid_structure() {
    assert!(is_valid_ulid(&ulidgen(None)));
}

/// Port of `test_ulid_uniqueness` from `tests/test.c`.
#[test]
fn test_ulid_uniqueness() {
    let a = ulidgen(None);
    let b = ulidgen(Some(&a));
    assert_ne!(a, b);
}

/// Port of `test_ulid_sortability` from `tests/test.c`.
#[test]
fn test_ulid_sortability() {
    let a = ulidgen(None);
    std::thread::sleep(Duration::from_micros(1500));
    let b = ulidgen(Some(&a));
    assert!(a < b);
}
