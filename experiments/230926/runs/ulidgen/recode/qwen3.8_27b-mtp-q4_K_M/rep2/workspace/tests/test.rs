//! Tests — port of `tests/test.c`.
//!
//! All four C tests are ported, including the structure test that was
//! commented out in the C `main` (it passes, so it is enabled here).

use std::time::Duration;

use ulidgen::ulid::{ulidgen_r, B32_ALPHABET};

/// Helper: validate ULID structure (C `is_valid_ulid`).
fn is_valid_ulid(ulid: &str) -> bool {
    ulid.len() == 26 && ulid.bytes().all(|b| B32_ALPHABET.contains(&b))
}

#[test]
fn test_ulid_length() {
    let mut u = [0u8; 27];
    ulidgen_r(&mut u);
    assert_eq!(std::str::from_utf8(&u[..26]).unwrap().len(), 26);
}

#[test]
fn test_ulid_structure() {
    let mut u = [0u8; 27];
    ulidgen_r(&mut u);
    assert!(is_valid_ulid(std::str::from_utf8(&u[..26]).unwrap()));
}

#[test]
fn test_ulid_uniqueness() {
    let mut u = [0u8; 27];
    ulidgen_r(&mut u);
    let first = std::str::from_utf8(&u[..26]).unwrap().to_string();
    ulidgen_r(&mut u);
    let second = std::str::from_utf8(&u[..26]).unwrap().to_string();
    assert_ne!(first, second);
}

#[test]
fn test_ulid_sortability() {
    let mut u = [0u8; 27];
    ulidgen_r(&mut u);
    let u1 = std::str::from_utf8(&u[..26]).unwrap().to_string();
    std::thread::sleep(Duration::from_micros(1_500));
    ulidgen_r(&mut u);
    let u2 = std::str::from_utf8(&u[..26]).unwrap().to_string();
    assert!(u1 < u2);
}
