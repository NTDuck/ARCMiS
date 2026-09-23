//! Integration tests — mirror of `tests/test.c`.

use ulidgen::UlidGen;

const B32: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Helper: validate ULID structure (C: `is_valid_ulid`).
fn is_valid_ulid(u: &str) -> bool {
    u.len() == 26 && u.bytes().all(|b| B32.as_bytes().contains(&b))
}

/// C: `test_ulid_length`
#[test]
fn ulid_length() {
    let mut g = UlidGen::new();
    assert_eq!(g.next().len(), 26, "ULID length should be 26 characters");
}

/// C: `test_ulid_structure`
#[test]
fn ulid_structure() {
    let mut g = UlidGen::new();
    assert!(is_valid_ulid(&g.next()), "ULID should only contain valid Base32 characters");
}

/// C: `test_ulid_uniqueness`
#[test]
fn ulid_uniqueness() {
    let mut g = UlidGen::new();
    assert_ne!(g.next(), g.next(), "Consecutive ULIDs should be unique");
}

/// C: `test_ulid_sortability`
#[test]
fn ulid_sortability() {
    let mut g = UlidGen::new();
    let a = g.next();
    // Simulate a delay to ensure different timestamps (C: 1.5 ms nanosleep).
    std::thread::sleep(std::time::Duration::from_millis(2));
    let b = g.next();
    assert!(a < b, "ULIDs should be lexicographically sortable based on time");
}
