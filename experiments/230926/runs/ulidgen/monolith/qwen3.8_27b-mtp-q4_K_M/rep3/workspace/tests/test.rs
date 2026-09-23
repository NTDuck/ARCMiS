use ulidgen::ulidgen_r;

// Helper function to validate ULID structure
fn is_valid_ulid(ulid: &str) -> bool {
    const B32: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    // Check length
    if ulid.len() != 26 {
        return false;
    }

    // Check characters
    ulid.bytes().all(|c| B32.contains(c as char))
}

fn to_str(ulid: &[u8; 27]) -> &str {
    std::str::from_utf8(&ulid[..26]).unwrap()
}

// Test Cases
#[test]
fn test_ulid_length() {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    assert_eq!(to_str(&ulid).len(), 26, "ULID length should be 26 characters");
}

#[test]
fn test_ulid_structure() {
    let mut ulid = [0u8; 27];
    ulidgen_r(&mut ulid);
    let s = to_str(&ulid);
    eprintln!("Generated ULID: {s}");
    assert!(is_valid_ulid(s), "ULID should only contain valid Base32 characters");
}

#[test]
fn test_ulid_uniqueness() {
    let mut ulid1 = [0u8; 27];
    let mut ulid2 = [0u8; 27];
    ulidgen_r(&mut ulid1);
    ulidgen_r(&mut ulid2);

    // Ensure two ULIDs generated consecutively are not the same
    assert_ne!(to_str(&ulid1), to_str(&ulid2), "Consecutive ULIDs should be unique");
}

#[test]
fn test_ulid_sortability() {
    let mut ulid1 = [0u8; 27];
    ulidgen_r(&mut ulid1);

    // Simulate a delay to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_nanos(1_500_000)); // 1.5 ms

    let mut ulid2 = [0u8; 27];
    ulidgen_r(&mut ulid2);
    assert!(
        to_str(&ulid1) < to_str(&ulid2),
        "ULIDs should be lexicographically sortable based on time"
    );
}
