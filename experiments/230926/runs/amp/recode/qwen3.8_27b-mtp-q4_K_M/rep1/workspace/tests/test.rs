//! Integration test mirroring `tests/test.c` exactly:
//! encode `["some", "stuff", "here"]`, assert version == 1 and argc == 3,
//! then decode the three args and compare them.

use amp::{encode, Message, VERSION};

#[test]
fn test() {
    let args = ["some", "stuff", "here"];

    // encode
    let buf = encode(&args).expect("encode should succeed");

    // header
    let mut msg = Message::decode(&buf);
    assert_eq!(VERSION, msg.version);
    assert_eq!(1, msg.version);
    assert_eq!(3, msg.argc);

    // args
    for (i, expected) in args.iter().enumerate() {
        let arg = msg
            .decode_arg()
            .unwrap_or_else(|| panic!("arg {i} should decode"));
        assert_eq!(expected.as_bytes(), arg);
    }
}
