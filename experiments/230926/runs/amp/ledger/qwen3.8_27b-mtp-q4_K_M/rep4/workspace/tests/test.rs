use amp::{encode, Amp};

#[test]
fn encode_decode_roundtrip() {
    let args: &[&[u8]] = &[b"some", b"stuff", b"here"];

    // encode
    let buf = encode(args);

    // header
    let mut msg = Amp::new(&buf).unwrap();
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 3);

    // args
    for expected in args {
        let arg = msg.decode_arg().unwrap();
        assert_eq!(arg.as_slice(), *expected);
    }
}
