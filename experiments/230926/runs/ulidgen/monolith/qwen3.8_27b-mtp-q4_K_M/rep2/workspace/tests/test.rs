use ulidgen::ulidgen_r;

/// Helper function to validate ULID structure
fn is_valid_ulid(ulid: &str) -> bool {
    const B32_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    // Check length
    if ulid.len() != 26 {
        return false;
    }

    // Check characters
    ulid.bytes().all(|c| B32_ALPHABET.contains(c as char))
}

fn gen() -> String {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    std::str::from_utf8(&ulid[..26]).unwrap().to_string()
}

// Test Cases
#[test]
fn test_ulid_length() {
    let ulid = gen();
    assert_eq!(ulid.len(), 26, "ULID length should be 26 characters");
}

#[test]
fn test_ulid_structure() {
    let ulid = gen();
    println!("Generated ULID: {}", ulid);
    assert!(is_valid_ulid(&ulid), "ULID should only contain valid Base32 characters");
}

#[test]
fn test_ulid_uniqueness() {
    let ulid1 = gen();
    let ulid2 = gen();

    // Ensure two ULIDs generated consecutively are not the same
    assert_ne!(ulid1, ulid2, "Consecutive ULIDs should be unique");
}

#[test]
fn test_ulid_sortability() {
    let ulid1 = gen();

    // Simulate a delay to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_nanos(1_500_000)); // 1.5 ms

    let ulid2 = gen();
    assert!(
        ulid1 < ulid2,
        "ULIDs should be lexicographically sortable based on time"
    );
}
