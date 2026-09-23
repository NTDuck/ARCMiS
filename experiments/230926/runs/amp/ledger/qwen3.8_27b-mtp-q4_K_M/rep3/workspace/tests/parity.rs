//! Byte-level parity check against the C implementation's wire format.
//!
//! amp_encode(["some","stuff","here"]) must produce EXACTLY:
//!   0x13,
//!   0x00,0x00,0x00,0x04, 0x73,0x6f,0x6d,0x65,
//!   0x00,0x00,0x00,0x05, 0x73,0x74,0x75,0x66,0x66,
//!   0x00,0x00,0x00,0x04, 0x68,0x65,0x72,0x65
//! (header 0x13 = version 1 in high nibble, argc 3 in low nibble;
//!  then u32be lengths 4,5,4 with payloads "some","stuff","here").

use amp::{amp_decode, amp_decode_arg, amp_encode, Amp};

#[test]
fn parity_encode_exact_bytes() {
    let args: [&[u8]; 3] = [b"some", b"stuff", b"here"];
    let buf = amp_encode(&args);

    let expected: &[u8] = &[
        0x13,
        0x00, 0x00, 0x00, 0x04, 0x73, 0x6f, 0x6d, 0x65,
        0x00, 0x00, 0x00, 0x05, 0x73, 0x74, 0x75, 0x66, 0x66,
        0x00, 0x00, 0x00, 0x04, 0x68, 0x65, 0x72, 0x65,
    ];

    assert_eq!(buf.len(), expected.len(), "encoded length mismatch");
    for (i, (got, want)) in buf.iter().zip(expected.iter()).enumerate() {
        assert_eq!(*got, *want, "byte {i} mismatch");
    }
}

#[test]
fn parity_decode_roundtrip_and_cursor() {
    let args: [&[u8]; 3] = [b"some", b"stuff", b"here"];
    let buf = amp_encode(&args);

    let mut msg = Amp::new();
    amp_decode(&mut msg, &buf);
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 3);

    let expected = [b"some".as_slice(), b"stuff".as_slice(), b"here".as_slice()];
    for (i, want) in expected.iter().enumerate() {
        let arg = amp_decode_arg(&mut msg);
        assert_eq!(&arg[..], *want, "arg {i} mismatch");
    }

    // cursor fully consumed after the last arg
    assert_eq!(msg.pos, msg.buf.len(), "cursor not fully consumed");
}
