//! Integration tests mirroring the C `tests/test.c`.
//!
//! The C tests reuse ONE buffer across `ulidgen_r` calls, which is what makes
//! the same-millisecond increment path (and thus `ulid_uniqueness`) work.
//! We do the same here via a shared buffer helper.

use ulidgen::{ulidgen_r, B32_ALPHABET};

/// Generate a ULID into the shared buffer (mirrors the C tests' single
/// `char ulid[27]` reused across calls).
fn gen(buf: &mut [u8; 27]) -> String {
    ulidgen_r(buf);
    String::from_utf8(buf[..26].to_vec()).unwrap()
}

/// Helper function to validate ULID structure (mirrors C `is_valid_ulid`).
fn is_valid_ulid(ulid: &str) -> bool {
    if ulid.len() != 26 {
        return false;
    }
    ulid.bytes().all(|c| B32_ALPHABET.as_bytes().contains(&c))
}

#[test]
fn ulid_length() {
    let mut buf = [0u8; 27];
    let ulid = gen(&mut buf);
    assert_eq!(ulid.len(), 26, "ULID length should be 26 characters");
}

#[test]
fn ulid_structure() {
    let mut buf = [0u8; 27];
    let ulid = gen(&mut buf);
    eprintln!("Generated ULID: {ulid}");
    assert!(is_valid_ulid(&ulid), "ULID should only contain valid Base32 characters");
}

#[test]
fn ulid_uniqueness() {
    let mut buf = [0u8; 27];
    let ulid1 = gen(&mut buf);
    let ulid2 = gen(&mut buf);
    assert_ne!(ulid1, ulid2, "Consecutive ULIDs should be unique");
}

#[test]
fn ulid_sortability() {
    let mut buf = [0u8; 27];
    let ulid1 = gen(&mut buf);

    // Simulate a delay to ensure different timestamps.
    // C used 1.5 ms; use 2 ms for safety on loaded CI.
    std::thread::sleep(std::time::Duration::from_millis(2));

    let ulid2 = gen(&mut buf);
    assert!(ulid1 < ulid2, "ULIDs should be lexicographically sortable based on time");
}
