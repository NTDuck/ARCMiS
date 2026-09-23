//! Coverage for the `encode` error path: `argv.len() > 15` must return
//! `Err(AmpError::ArgcTooLarge)` (the protocol's 4-bit argc field cannot
//! represent the count). The boundary (argc == 15) must still succeed and
//! stay byte-identical to the C wire format.

use amp::{encode, AmpError, Message, VERSION};

/// 16 arguments exceed the 4-bit argc field: `encode` must fail with
/// `AmpError::ArgcTooLarge { argc: 16 }` and produce no buffer.
#[test]
fn encode_rejects_16_args() {
    let args: Vec<&str> = (0..16).map(|i| Box::leak(Box::new(format!("arg{i}"))).as_str()).collect();
    let err = encode(&args).expect_err("argc 16 must be rejected");
    assert_eq!(err, AmpError::ArgcTooLarge { argc: 16 });
}

/// 17 arguments: the error carries the actual count.
#[test]
fn encode_rejects_17_args_with_count() {
    let args: Vec<&str> = (0..17).map(|i| Box::leak(Box::new(format!("a{i}"))).as_str()).collect();
    let err = encode(&args).expect_err("argc 17 must be rejected");
    assert_eq!(err, AmpError::ArgcTooLarge { argc: 17 });
}

/// Boundary: exactly 15 arguments is representable (header low nibble 0xf)
/// and must succeed with the correct header byte.
#[test]
fn encode_accepts_exactly_15_args() {
    let args: Vec<&str> = (0..15).map(|i| Box::leak(Box::new(format!("x{i}"))).as_str()).collect();
    let buf = encode(&args).expect("argc 15 must be accepted");
    // Header: version 1 in high nibble, argc 15 in low nibble.
    assert_eq!(buf[0], (VERSION << 4) | 15);
    assert_eq!(buf[0], 0x1f);

    // Round-trip: all 15 args decode back in order.
    let mut msg = Message::decode(&buf);
    assert_eq!(msg.version, VERSION);
    assert_eq!(msg.argc, 15);
    for expected in &args {
        let arg = msg.decode_arg().expect("arg should decode");
        assert_eq!(expected.as_bytes(), arg);
    }
    assert!(msg.decode_arg().is_none(), "no 16th arg should exist");
}

/// The error must be printable with a message naming the offending count.
#[test]
fn argc_too_large_display_names_count() {
    let err = AmpError::ArgcTooLarge { argc: 16 };
    let msg = err.to_string();
    assert!(msg.contains("16"), "display should mention the count: {msg}");
    assert!(msg.contains("15"), "display should mention the limit: {msg}");
}
