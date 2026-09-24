//! Port of C `tests/test.c` plus edge-case tests the C version lacked.

use amp::{encode, AmpError, AmpMessage};

/// Direct port of `tests/test.c` main():
/// encode ["some", "stuff", "here"], verify header, decode all 3 args.
#[test]
fn round_trip() {
    let args: Vec<&[u8]> = vec![b"some", b"stuff", b"here"];
    let buf = encode(&args).unwrap();
    let mut msg = AmpMessage::new(&buf).unwrap();
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 3);
    assert_eq!(msg.decode_arg().unwrap(), b"some");
    assert_eq!(msg.decode_arg().unwrap(), b"stuff");
    assert_eq!(msg.decode_arg().unwrap(), b"here");
}

/// Truncated buffer must yield `AmpError::TruncatedArgument`, not UB.
#[test]
fn truncated_buffer() {
    // Encode one arg, then chop the buffer mid-argument:
    // keep the header + only 2 of the 4 length-prefix bytes.
    let buf = encode(&[b"some"]).unwrap();
    let truncated = &buf[..3];
    let mut msg = AmpMessage::new(truncated).unwrap();
    assert_eq!(msg.decode_arg(), Err(AmpError::TruncatedArgument));

    // Also: full length prefix but not enough data bytes.
    let buf = encode(&[b"some"]).unwrap();
    let truncated = &buf[..6]; // header + 4-byte length + 1 data byte
    let mut msg = AmpMessage::new(truncated).unwrap();
    assert_eq!(msg.decode_arg(), Err(AmpError::TruncatedArgument));

    // Empty input is a distinct error.
    assert!(matches!(AmpMessage::new(b""), Err(AmpError::EmptyBuffer)));
}

/// `encode` with 16 args must be rejected (4-bit argc field).
#[test]
fn too_many_args() {
    let data: Vec<u8> = (0..16).collect();
    let args: Vec<&[u8]> = data.iter().map(|b| std::slice::from_ref(b)).collect();
    assert_eq!(encode(&args), Err(AmpError::TooManyArgs));
}

/// Empty argv (argc = 0) round-trips: 1-byte buffer, no args.
#[test]
fn empty_argv() {
    let buf = encode(&[]).unwrap();
    assert_eq!(buf, vec![0x10]);
    let msg = AmpMessage::new(&buf).unwrap();
    assert_eq!(msg.version, 1);
    assert_eq!(msg.argc, 0);
}

/// Arguments may contain arbitrary bytes (e.g. embedded NUL) —
/// the length-prefixed format supports it; C strcmp-based tests could not.
#[test]
fn embedded_nul_bytes() {
    let buf = encode(&[b"a\0b"]).unwrap();
    let mut msg = AmpMessage::new(&buf).unwrap();
    assert_eq!(msg.argc, 1);
    assert_eq!(msg.decode_arg().unwrap(), b"a\0b");
}
