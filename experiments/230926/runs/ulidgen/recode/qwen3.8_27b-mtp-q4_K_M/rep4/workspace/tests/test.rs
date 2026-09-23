//! Integration tests — port of C `tests/test.c`.
//!
//! Mirrors: test_ulid_length, test_ulid_structure, test_ulid_uniqueness,
//! test_ulid_sortability (Rust test names drop the `test_` prefix).

use ulidgen::ulidgen_r;

/// Helper: validate ULID structure (26 chars, all in the Crockford Base32
/// alphabet). Port of C `is_valid_ulid`.
fn is_valid_ulid(ulid: &[u8]) -> bool {
    const A: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    ulid.len() == 26 && ulid.iter().all(|c| A.contains(c))
}

/// Port of C `test_ulid_length`: ULID is 26 chars, NUL at [26].
#[test]
fn ulid_length() {
    let mut u = [0u8; 27];
    ulidgen_r(&mut u);
    assert_eq!(u[26], 0);
    assert!(is_valid_ulid(&u[..26]));
}

/// Port of C `test_ulid_structure`: all 26 chars are valid Base32.
#[test]
fn ulid_structure() {
    let mut u = [0u8; 27];
    ulidgen_r(&mut u);
    assert!(is_valid_ulid(&u[..26]));
}

/// Port of C `test_ulid_uniqueness`: two consecutive calls (separate
/// buffers) differ.
#[test]
fn ulid_uniqueness() {
    let mut a = [0u8; 27];
    let mut b = [0u8; 27];
    ulidgen_r(&mut a);
    ulidgen_r(&mut b);
    assert_ne!(&a[..26], &b[..26]);
}

/// Port of C `test_ulid_sortability`: after a >1.5 ms sleep (use 2 ms for
/// margin), the first ULID sorts before the second.
#[test]
fn ulid_sortability() {
    let mut a = [0u8; 27];
    let mut b = [0u8; 27];
    ulidgen_r(&mut a);
    std::thread::sleep(std::time::Duration::from_millis(2));
    ulidgen_r(&mut b);
    let sa = std::str::from_utf8(&a[..26]).unwrap();
    let sb = std::str::from_utf8(&b[..26]).unwrap();
    assert!(sa < sb);
}
