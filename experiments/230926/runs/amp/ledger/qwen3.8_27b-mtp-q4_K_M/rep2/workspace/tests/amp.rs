//! Port of `tests/test.c`.

use amp::{amp_decode, amp_decode_arg, amp_encode};

#[test]
fn encode_decode_round_trip() {
    let args = ["some", "stuff", "here"];
    let buf = amp_encode(&args);

    let mut msg = amp_decode(&buf);
    assert_eq!(1, msg.version);
    assert_eq!(3, msg.argc);

    for i in 0..msg.argc {
        let arg = amp_decode_arg(&mut msg);
        let arg = String::from_utf8(arg).expect("arg bytes are valid UTF-8");
        match i {
            0 => assert_eq!("some", arg),
            1 => assert_eq!("stuff", arg),
            2 => assert_eq!("here", arg),
            _ => unreachable!(),
        }
    }
}
