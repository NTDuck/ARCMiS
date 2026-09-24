//! Integration test mirroring `tests/test.c` from the C project.

use amp::{decode, decode_arg, encode, AmpMessage, VERSION};

#[test]
fn roundtrip() {
    // C: char *args[] = { "some", "stuff", "here" };
    let args: Vec<&[u8]> = ["some", "stuff", "here"]
        .iter()
        .map(|s| s.as_bytes())
        .collect();

    // encode
    let buf = encode(&args);

    // header
    let mut msg = AmpMessage::default();
    decode(&mut msg, &buf).unwrap();
    assert_eq!(msg.version, VERSION);
    assert_eq!(msg.argc, 3);

    // args
    for expected in ["some", "stuff", "here"] {
        let arg = decode_arg(&mut msg).unwrap();
        assert_eq!(arg, expected.as_bytes());
    }
}
