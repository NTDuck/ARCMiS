//! Integration tests — port of `tests/test.c` (public domain, Leah Neukirchen).
//!
//! All four C tests are real `#[test]`s here (the C `main` never called
//! `test_ulid_structure`; running it is strictly more coverage).

use ulidgen::ulidgen_r;

/// Helper function to validate ULID structure (port of `is_valid_ulid`).
fn is_valid_ulid(ulid: &str) -> bool {
    const A: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    ulid.len() == 26 && ulid.bytes().all(|c| A.as_bytes().contains(&c))
}

#[test]
fn ulid_length() {
    // port of test_ulid_length: strlen == 26
    let mut u = [0u8; 27];
    ulidgen_r(&mut u);
    assert_eq!(std::str::from_utf8(&u[..26]).unwrap().len(), 26);
}

#[test]
fn ulid_structure() {
    // port of test_ulid_structure: all chars in the base32 alphabet
    let mut u = [0u8; 27];
    ulidgen_r(&mut u);
    assert!(is_valid_ulid(std::str::from_utf8(&u[..26]).unwrap()));
}

#[test]
fn ulid_uniqueness() {
    // port of test_ulid_uniqueness: two consecutive ULIDs differ
    let mut a = [0u8; 27];
    let mut b = [0u8; 27];
    ulidgen_r(&mut a);
    ulidgen_r(&mut b);
    assert_ne!(
        std::str::from_utf8(&a[..26]).unwrap(),
        std::str::from_utf8(&b[..26]).unwrap()
    );
}

#[test]
fn ulid_sortability() {
    // port of test_ulid_sortability: after a 1.5 ms sleep, second ULID > first
    let mut a = [0u8; 27];
    let mut b = [0u8; 27];
    ulidgen_r(&mut a);
    std::thread::sleep(std::time::Duration::from_micros(1500));
    ulidgen_r(&mut b);
    assert!(std::str::from_utf8(&a[..26]).unwrap() < std::str::from_utf8(&b[..26]).unwrap());
}
