use amp::{amp_decode, amp_encode};

#[test]
fn test_roundtrip() {
    let args = ["some", "stuff", "here"];
    let buf = amp_encode(&args);
    let mut msg = amp_decode(&buf);
    assert_eq!(1, msg.version);
    assert_eq!(3, msg.argc);
    for i in 0..msg.argc {
        let arg = msg.amp_decode_arg();
        let expected = match i {
            0 => "some",
            1 => "stuff",
            2 => "here",
            _ => unreachable!(),
        };
        assert_eq!(expected, String::from_utf8(arg).unwrap());
    }
}
