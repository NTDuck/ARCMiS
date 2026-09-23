//! Port of `tests/test.c` — run with `cargo test`.

/// Helper: validate ULID structure (port of C `is_valid_ulid`).
fn is_valid_ulid(ulid: &str) -> bool {
    todo!("port of C is_valid_ulid: length == 26 and every char in the Crockford base32 alphabet")
}

#[test]
fn test_ulid_length() {
    todo!("port: generated ULID has length 26")
}

#[test]
fn test_ulid_structure() {
    todo!("port: generated ULID contains only valid Base32 characters (is_valid_ulid)")
}

#[test]
fn test_ulid_uniqueness() {
    todo!("port: two ULIDs generated consecutively are not the same")
}

#[test]
fn test_ulid_sortability() {
    todo!("port: after a 1.5 ms sleep, the second ULID sorts after the first (lexicographic comparison)")
}
