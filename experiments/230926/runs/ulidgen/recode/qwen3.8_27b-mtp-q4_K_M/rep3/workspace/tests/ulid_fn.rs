//! Tests for the `ulid()` convenience wrapper (fresh-buffer semantics).

use ulidgen::{ulid, B32_ALPHABET};

fn is_valid_ulid(ulid: &str) -> bool {
    ulid.len() == 26 && ulid.bytes().all(|c| B32_ALPHABET.as_bytes().contains(&c))
}

#[test]
fn ulid_returns_26_valid_chars() {
    let u = ulid();
    assert_eq!(u.len(), 26, "ulid() must return exactly 26 characters");
    assert!(is_valid_ulid(&u), "ulid() must contain only Crockford base32 chars: {u}");
}

#[test]
fn ulid_is_unique_across_calls() {
    // Fresh buffers take the random path each time (128 bits of entropy),
    // so two calls must differ.
    let a = ulid();
    let b = ulid();
    assert_ne!(a, b, "consecutive ulid() calls should differ");
}
