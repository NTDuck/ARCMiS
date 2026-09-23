//! Tests translated from the C test suite: length, structure,
//! uniqueness, and sortability of generated ULIDs.

use ulidgen::UlidGen;

const B32: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

#[test]
fn length() {
    let mut gen = UlidGen::new();
    for _ in 0..100 {
        let ulid = gen.next();
        assert_eq!(ulid.len(), 26, "ULID must be 26 chars: {ulid}");
    }
}

#[test]
fn structure() {
    let mut gen = UlidGen::new();
    for _ in 0..100 {
        let ulid = gen.next();
        for c in ulid.bytes() {
            assert!(
                B32.contains(&c),
                "ULID contains char not in Crockford base32 alphabet: {ulid}"
            );
        }
    }
}

#[test]
fn uniqueness() {
    let mut gen = UlidGen::new();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..1000 {
        let ulid = gen.next();
        assert!(seen.insert(ulid), "duplicate ULID generated");
    }
}

#[test]
fn sortability() {
    // ULIDs generated in order must be lexicographically non-decreasing.
    let mut gen = UlidGen::new();
    let mut prev = gen.next();
    for _ in 0..1000 {
        let ulid = gen.next();
        assert!(
            ulid >= prev,
            "ULIDs out of order: {prev} then {ulid}"
        );
        prev = ulid;
    }
}

#[test]
fn timestamp_prefix_tracks_time() {
    // The first 10 chars encode the ms timestamp; decode and check it is
    // close to now.
    let ulid = UlidGen::new().next();
    let mut t: u64 = 0;
    for c in ulid[..10].bytes() {
        let d = B32.iter().position(|&x| x == c).expect("valid alphabet char") as u64;
        t = t * 32 + d;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    assert!(
        now >= t && now - t < 1000,
        "timestamp {t} not within 1s of now {now}"
    );
}
