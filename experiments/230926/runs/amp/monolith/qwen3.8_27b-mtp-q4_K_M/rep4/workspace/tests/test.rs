use amp::{amp_decode, amp_decode_arg, amp_encode, Amp};

#[test]
fn test() {
    let args = ["some", "stuff", "here"];

    // encode
    let buf = amp_encode(&args);

    // header
    let mut msg = Amp::default();
    amp_decode(&mut msg, &buf);
    assert_eq!(1, msg.version);
    assert_eq!(3, msg.argc);

    // args
    for i in 0..msg.argc {
        let arg = amp_decode_arg(&mut msg);
        match i {
            0 => assert_eq!("some", String::from_utf8(arg).unwrap()),
            1 => assert_eq!("stuff", String::from_utf8(arg).unwrap()),
            2 => assert_eq!("here", String::from_utf8(arg).unwrap()),
            _ => unreachable!(),
        }
    }

    println!("ok");
}
