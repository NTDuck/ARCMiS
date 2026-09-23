//! Round-trip tests. Mirrors C `tests/test.c`, plus robustness tests.

use amp::{encode, Message, VERSION};

/// Mirrors `tests/test.c` main(): encode `["some", "stuff", "here"]`,
/// decode the header, decode each argument, assert equality.
#[test]
fn round_trip() {
    let args = ["some", "stuff", "here"];
    let buf = encode(&args).unwrap();
    // Wire-format guard: header byte for version 1, 3 args is 0x13.
    assert_eq!(0x13, buf[0]);
    let mut msg = Message::decode(&buf);
    assert_eq!(VERSION, msg.version);
    assert_eq!(3, msg.argc);
    assert_eq!(Some("some".to_string()), msg.decode_arg());
    assert_eq!(Some("stuff".to_string()), msg.decode_arg());
    assert_eq!(Some("here".to_string()), msg.decode_arg());
    assert_eq!(None, msg.decode_arg()); // exhausted
}

/// Deliberate behavior change vs C: argc > 15 must be rejected,
/// not silently corrupt the header.
#[test]
fn rejects_too_many_args() {
    let args: Vec<&str> = (0..16).map(|i| &*format!("a{i}").leak()).collect();
    assert!(encode(&args).is_none());
}
