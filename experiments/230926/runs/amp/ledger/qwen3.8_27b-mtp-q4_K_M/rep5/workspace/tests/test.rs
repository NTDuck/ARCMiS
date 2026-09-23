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
            0 => assert_eq!(b"some".to_vec(), arg),
            1 => assert_eq!(b"stuff".to_vec(), arg),
            2 => assert_eq!(b"here".to_vec(), arg),
            _ => unreachable!(),
        }
    }

    println!("ok");
}

#[test]
fn test_byte_exact() {
    let buf = amp_encode(&["some", "stuff", "here"]);
    let expected: Vec<u8> = vec![
        0x13, // version 1, argc 3
        0, 0, 0, 4, b's', b'o', b'm', b'e',
        0, 0, 0, 5, b's', b't', b'u', b'f', b'f',
        0, 0, 0, 4, b'h', b'e', b'r', b'e',
    ];
    assert_eq!(expected, buf);
}
