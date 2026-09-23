use amp::{amp_decode, amp_decode_arg, amp_encode, AmpMessage};

#[test]
fn test() {
    let args = ["some", "stuff", "here"];

    // encode
    let buf = amp_encode(&args);

    // header
    let mut msg = AmpMessage::default();
    amp_decode(&mut msg, &buf);
    assert_eq!(1, msg.version);
    assert_eq!(3, msg.argc);

    // args
    for expected in args.iter() {
        let arg = amp_decode_arg(&mut msg);
        assert_eq!(expected.as_bytes(), &arg[..]);
    }
}
