//! Integration tests — port of tests/test.c.
//!
//! Uses the public API of the `ulidgen` crate.

use ulidgen::ulid;

/// Port of the `is_valid_ulid` helper in tests/test.c.
fn is_valid_ulid(ulid: &str) -> bool {
    // TODO: implement
    const B32_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    ulid.len() == 26 && ulid.bytes().all(|b| B32_ALPHABET.as_bytes().contains(&b))
}

/// Port of `test_ulid_length`.
#[test]
fn ulid_length() {
    // TODO: implement
    assert_eq!(ulid().len(), 26);
}

/// Port of `test_ulid_structure`.
#[test]
fn ulid_structure() {
    // TODO: implement
    let u = ulid();
    println!("Generated ULID: {u}");
    assert!(is_valid_ulid(&u));
}

/// Port of `test_ulid_uniqueness`.
#[test]
fn ulid_uniqueness() {
    // TODO: implement
    assert_ne!(ulid(), ulid());
}

/// Port of `test_ulid_sortability` (sleep 2 ms to be safe, vs 1.5 ms in C).
#[test]
fn ulid_sortability() {
    // TODO: implement
    let a = ulid();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let b = ulid();
    assert!(a < b);
}
