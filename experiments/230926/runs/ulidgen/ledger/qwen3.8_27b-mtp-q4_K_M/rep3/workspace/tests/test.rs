/*
 * Port of tests/test.c for the Rust ulidgen library.
 *
 * To the extent possible under law, Leah Neukirchen <leah@vuxu.org>
 * has waived all copyright and related or neighboring rights to this work.
 * http://creativecommons.org/publicdomain/zero/1.0/
 */

use ulidgen::ulidgen_r;

const B32_ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/* Helper function to validate ULID structure */
fn is_valid_ulid(ulid: &[u8; 27]) -> bool {
    // Check length: first 26 bytes non-NUL, byte 26 is NUL
    if ulid[26] != 0 {
        return false;
    }
    for i in 0..26 {
        if ulid[i] == 0 || !B32_ALPHABET.contains(&ulid[i]) {
            return false;
        }
    }
    true
}

#[test]
fn test_ulid_length() {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    assert_eq!(ulid[26], 0, "byte 26 must be NUL");
    assert!(
        (0..26).all(|i| ulid[i] != 0),
        "ULID length should be 26 characters"
    );
}

#[test]
fn test_ulid_structure() {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    let s = String::from_utf8_lossy(&ulid[..26]);
    println!("Generated ULID: {s}");
    assert!(
        is_valid_ulid(&ulid),
        "ULID should only contain valid Base32 characters"
    );
}

#[test]
fn test_ulid_uniqueness() {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    let first = ulid[..26].to_vec();
    ulidgen_r(&mut ulid);
    assert_ne!(
        first,
        ulid[..26],
        "Consecutive ULIDs should be unique"
    );
}

#[test]
fn test_ulid_sortability() {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    let first = ulid[..26].to_vec();

    // Simulate a delay to ensure different timestamps (1.5 ms)
    std::thread::sleep(std::time::Duration::from_nanos(1_500_000));

    ulidgen_r(&mut ulid);
    assert!(
        first.as_slice() < &ulid[..26],
        "ULIDs should be lexicographically sortable based on time"
    );
}
