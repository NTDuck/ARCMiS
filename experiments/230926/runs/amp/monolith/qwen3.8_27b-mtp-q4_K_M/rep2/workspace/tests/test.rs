use amp::{encode, Amp};

#[test]
fn test() {
    let args: Vec<&[u8]> = vec![b"some", b"stuff", b"here"];

    // encode
    let buf = encode(&args);

    // header
    let mut msg = Amp::decode(&buf);
    assert_eq!(1, msg.version);
    assert_eq!(3, msg.argc);

    // args
    for i in 0..msg.argc {
        let arg = msg.decode_arg().unwrap();
        match i {
            0 => assert_eq!(b"some", arg.as_slice()),
            1 => assert_eq!(b"stuff", arg.as_slice()),
            2 => assert_eq!(b"here", arg.as_slice()),
            _ => unreachable!(),
        }
    }

    println!("ok");
}
