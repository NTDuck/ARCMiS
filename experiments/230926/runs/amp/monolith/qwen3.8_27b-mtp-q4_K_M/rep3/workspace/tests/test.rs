use amp::{amp_decode, amp_decode_arg, amp_encode, AmpMsg};

#[test]
fn test() {
    let args = ["some", "stuff", "here"];

    // encode
    let buf = amp_encode(&args);

    // header
    let mut msg = AmpMsg::default();
    amp_decode(&mut msg, &buf);
    assert_eq!(1, msg.version);
    assert_eq!(3, msg.argc);

    // args
    for i in 0..msg.argc {
        let arg = amp_decode_arg(&mut msg);
        let arg = String::from_utf8(arg).unwrap();
        match i {
            0 => assert_eq!("some", arg),
            1 => assert_eq!("stuff", arg),
            2 => assert_eq!("here", arg),
            _ => unreachable!(),
        }
    }

    println!("ok");
}
