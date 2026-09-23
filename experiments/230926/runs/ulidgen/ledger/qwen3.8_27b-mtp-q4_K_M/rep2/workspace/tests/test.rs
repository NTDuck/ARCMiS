//! Port of tests/test.c to a Rust integration test.

use std::thread;
use std::time::Duration;

use ulidgen::ulidgen_r;

/// Helper function to validate ULID structure
fn is_valid_ulid(ulid: &str) -> bool {
    const B32ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    // Check length
    if ulid.len() != 26 {
        return false;
    }

    // Check characters
    for c in ulid.chars() {
        if !B32ALPHABET.contains(c) {
            return false;
        }
    }

    true
}

/// Convert the 27-byte buffer (26 chars + NUL) to a 26-char string.
fn ulid_str(buf: &[u8; 27]) -> String {
    String::from_utf8_lossy(&buf[0..26]).into_owned()
}

// Test Cases
#[test]
fn test_ulid_length() {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    assert_eq!(ulid_str(&ulid).len(), 26, "ULID length should be 26 characters");
}

#[test]
fn test_ulid_structure() {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    let s = ulid_str(&ulid);
    println!("Generated ULID: {}", s);
    assert!(is_valid_ulid(&s), "ULID should only contain valid Base32 characters");
}

#[test]
fn test_ulid_uniqueness() {
    let mut ulid1 = [0u8; 27];
    let mut ulid2 = [0u8; 27];
    ulidgen_r(&mut ulid1);
    ulidgen_r(&mut ulid2);

    // Ensure two ULIDs generated consecutively are not the same
    assert_ne!(ulid_str(&ulid1), ulid_str(&ulid2), "Consecutive ULIDs should be unique");
}

#[test]
fn test_ulid_sortability() {
    let mut ulid1 = [0u8; 27];
    let mut ulid2 = [0u8; 27];
    ulidgen_r(&mut ulid1);

    // Simulate a delay to ensure different timestamps
    thread::sleep(Duration::from_nanos(1_500_000)); // 1.5 ms

    ulidgen_r(&mut ulid2);
    assert!(
        ulid_str(&ulid1) < ulid_str(&ulid2),
        "ULIDs should be lexicographically sortable based on time"
    );
}
