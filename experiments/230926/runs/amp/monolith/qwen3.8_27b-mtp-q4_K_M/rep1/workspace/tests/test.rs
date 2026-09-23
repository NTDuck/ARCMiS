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
        let arg = amp_decode_arg(&mut msg).expect("decode arg");
        match i {
            0 => assert_eq!(b"some", &arg[..]),
            1 => assert_eq!(b"stuff", &arg[..]),
            2 => assert_eq!(b"here", &arg[..]),
            _ => unreachable!(),
        }
    }

    println!("ok");
}
