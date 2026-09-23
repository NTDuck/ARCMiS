//! Integration test — direct port of `tests/test.c`.
//!
//! Encodes `["some", "stuff", "here"]`, decodes the header (assert version == 1,
//! argc == 3), then decodes the three arguments and asserts their values.

use amp::{encode, AmpMessage};

#[test]
fn encode_decode_three_args() {
    let args: Vec<&[u8]> = vec!["some".as_bytes(), "stuff".as_bytes(), "here".as_bytes()];

    // encode
    let buf = encode(&args).expect("encode should succeed");

    // header
    let mut msg = AmpMessage::decode(&buf).expect("decode should succeed");
    assert_eq!(1, msg.version);
    assert_eq!(3, msg.argc);

    // args
    let expected = ["some", "stuff", "here"];
    for exp in expected {
        let arg = msg.decode_arg().expect("decode_arg should succeed");
        assert_eq!(exp.as_bytes(), arg.as_slice());
    }
}
