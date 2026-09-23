//! Port of `tests/test.c` (public domain, Leah Neukirchen).
//!
//! Each C test function becomes a `#[test]`. The C tests used fresh
//! (uninitialized) `char ulid[27]` locals; the Rust equivalent is a fresh
//! zeroed `[0u8; 27]` buffer per call, which in practice always takes the
//! re-randomize path, just like the C tests.

use std::thread;
use std::time::Duration;

use ulidgen_r::ulidgen_r;

/// Crockford-style base32 alphabet (no I, L, O, U).
const B32_ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Helper function to validate ULID structure (port of `is_valid_ulid`).
fn is_valid_ulid(ulid: &str) -> bool {
    // Check length
    if ulid.len() != 26 {
        return false;
    }

    // Check characters
    for &c in ulid.as_bytes() {
        if !B32_ALPHABET.contains(&c) {
            return false;
        }
    }

    true
}

/// Convert the `[u8; 27]` buffer to a `String`, taking the bytes up to the
/// NUL terminator (or the first 26 bytes).
fn to_string(buf: &[u8; 27]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(26).min(26);
    String::from_utf8(buf[..end].to_vec()).expect("ULID is ASCII")
}

// Test Cases

#[test]
fn test_ulid_length() {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    assert_eq!(to_string(&ulid).len(), 26, "ULID length should be 26 characters");
}

#[test]
fn test_ulid_structure() {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    let ulid = to_string(&ulid);
    println!("Generated ULID: {ulid}");
    assert!(is_valid_ulid(&ulid), "ULID should only contain valid Base32 characters");
}

#[test]
fn test_ulid_uniqueness() {
    let mut ulid1 = [0u8; 27];
    let mut ulid2 = [0u8; 27];
    ulidgen_r(&mut ulid1);
    ulidgen_r(&mut ulid2);

    // Ensure two ULIDs generated consecutively are not the same
    assert_ne!(to_string(&ulid1), to_string(&ulid2), "Consecutive ULIDs should be unique");
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
        to_string(&ulid1) < to_string(&ulid2),
        "ULIDs should be lexicographically sortable based on time"
    );
}
