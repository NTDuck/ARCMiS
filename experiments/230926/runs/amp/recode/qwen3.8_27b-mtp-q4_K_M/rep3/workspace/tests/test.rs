//! Mirrors C `tests/test.c` 1:1: encode 3 args, assert version == 1,
//! argc == 3, and the three decoded args "some" / "stuff" / "here".

use amp::{decode, encode, VERSION};

#[test]
fn encode_decode_roundtrip() {
    let args: Vec<&[u8]> = vec![b"some", b"stuff", b"here"];

    // encode
    let buf = encode(&args).unwrap();

    // header
    let mut msg = decode(&buf).unwrap();
    assert_eq!(1, msg.version);
    assert_eq!(VERSION, msg.version);
    assert_eq!(3, msg.argc);

    // args
    assert_eq!(b"some", msg.arg().unwrap());
    assert_eq!(b"stuff", msg.arg().unwrap());
    assert_eq!(b"here", msg.arg().unwrap());
    assert!(msg.arg().is_none());
}
