use ulidgen::ulidgen_r;

// Helper function to validate ULID structure
fn is_valid_ulid(ulid: &str) -> bool {
    const B32_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    // Check length
    if ulid.len() != 26 {
        return false;
    }

    // Check characters
    ulid.bytes().all(|c| B32_ALPHABET.as_bytes().contains(&c))
}

fn ulid_to_string(ulid: &mut [u8; 27]) -> String {
    String::from_utf8_lossy(&ulid[..26]).into_owned()
}

// Test Cases
#[test]
fn test_ulid_length() {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    assert_eq!(
        ulid_to_string(&mut ulid).len(),
        26,
        "ULID length should be 26 characters"
    );
}

#[test]
fn test_ulid_structure() {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    let s = ulid_to_string(&mut ulid);
    println!("Generated ULID: {s}");
    assert!(
        is_valid_ulid(&s),
        "ULID should only contain valid Base32 characters"
    );
}

#[test]
fn test_ulid_uniqueness() {
    let mut ulid1 = [0u8; 27];
    let mut ulid2 = [0u8; 27];
    ulidgen_r(&mut ulid1);
    ulidgen_r(&mut ulid2);

    // Ensure two ULIDs generated consecutively are not the same
    assert_ne!(
        ulid_to_string(&mut ulid1),
        ulid_to_string(&mut ulid2),
        "Consecutive ULIDs should be unique"
    );
}

#[test]
fn test_ulid_sortability() {
    let mut ulid1 = [0u8; 27];
    let mut ulid2 = [0u8; 27];
    ulidgen_r(&mut ulid1);

    // Simulate a delay to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_micros(1500)); // 1.5 ms

    ulidgen_r(&mut ulid2);
    assert!(
        ulid_to_string(&mut ulid1) < ulid_to_string(&mut ulid2),
        "ULIDs should be lexicographically sortable based on time"
    );
}
