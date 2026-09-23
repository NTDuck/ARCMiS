//! AMP — Abstract Message Protocol.
//!
//! Encodes an argv-style message into a binary buffer and decodes it back.
//!
//! Wire format:
//! ```text
//! ------------+------------+------------+ ...
//! | <ver/argc> | <length>   | <data>     | additional args
//! | 1 byte     | 4 bytes BE | len bytes  |
//! ------------+------------+------------+ ...
//! ```
//! Byte 0: high nibble = protocol version, low nibble = argc (0–15).
//! Each argument: 4-byte big-endian length followed by raw bytes.

/// Protocol version (packed into the high nibble of the header byte).
pub const VERSION: u8 = 1;

/// A decoded AMP message with a cursor over the remaining argument bytes.
///
/// Borrows the encoded buffer for the lifetime `'a`.
pub struct AmpMessage<'a> {
    /// Protocol version (high nibble of the header byte).
    pub version: u8,
    /// Number of arguments (low nibble of the header byte).
    pub argc: u8,
    /// Cursor: remaining undecoded argument bytes.
    buf: &'a [u8],
}

impl<'a> AmpMessage<'a> {
    /// Decode the 1-byte header in `buf`, leaving the cursor at the args.
    ///
    /// Returns `None` if `buf` is empty (no header byte to read).
    pub fn decode(buf: &'a [u8]) -> Option<AmpMessage<'a>> {
        let header = *buf.first()?;
        let version = (header >> 4) & 0x0f;
        let argc = header & 0x0f;
        Some(AmpMessage {
            version,
            argc,
            buf: &buf[1..],
        })
    }

    /// Decode the next argument (u32be length + data), advancing the cursor.
    ///
    /// Returns an owned `Vec<u8>` copy of the argument bytes, or `None` if the
    /// buffer is truncated (length field or data runs past the end).
    pub fn decode_arg(&mut self) -> Option<Vec<u8>> {
        // Need at least 4 bytes for the length field.
        if self.buf.len() < 4 {
            return None;
        }
        let len = u32::from_be_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]);
        let len = len as usize;
        // Need the full data payload after the length field.
        if self.buf.len() < 4 + len {
            return None;
        }
        let data = self.buf[4..4 + len].to_vec();
        self.buf = &self.buf[4 + len..];
        Some(data)
    }
}

/// Encode an argv into an AMP message buffer.
///
/// Returns `None` if `argv.len() > 15` (argc does not fit in the header nibble).
pub fn encode(argv: &[&[u8]]) -> Option<Vec<u8>> {
    let argc = argv.len();
    if argc > 15 {
        return None;
    }
    let header = (VERSION << 4) | (argc as u8);
    let mut out: Vec<u8> = Vec::with_capacity(1 + argv.iter().map(|a| 4 + a.len()).sum::<usize>());
    out.push(header);
    for arg in argv {
        out.extend_from_slice(&(arg.len() as u32).to_be_bytes());
        out.extend_from_slice(arg);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_zero_args() {
        let buf = encode(&[]).expect("encode should succeed");
        let mut msg = AmpMessage::decode(&buf).expect("decode should succeed");
        assert_eq!(1, msg.version);
        assert_eq!(0, msg.argc);
        assert!(msg.decode_arg().is_none());
    }

    #[test]
    fn round_trip_max_args() {
        let owned: Vec<String> = (0..15).map(|i| format!("arg{}", i)).collect();
        let args: Vec<&[u8]> = owned.iter().map(|s| s.as_bytes()).collect();
        let buf = encode(&args).expect("encode should succeed");
        let mut msg = AmpMessage::decode(&buf).expect("decode should succeed");
        assert_eq!(1, msg.version);
        assert_eq!(15, msg.argc);
        for exp in &args {
            let arg = msg.decode_arg().expect("decode_arg should succeed");
            assert_eq!(*exp, arg.as_slice());
        }
    }

    #[test]
    fn encode_too_many_args_returns_none() {
        let owned: Vec<String> = (0..16).map(|i| format!("arg{}", i)).collect();
        let args: Vec<&[u8]> = owned.iter().map(|s| s.as_bytes()).collect();
        assert!(encode(&args).is_none());
    }

    #[test]
    fn decode_arg_truncated_returns_none() {
        // Header (argc=1) + a length field claiming 10 bytes, but only 2 present.
        let buf: Vec<u8> = vec![0x11, 0x00, 0x00, 0x00, 0x0a, 0x61, 0x62];
        let mut msg = AmpMessage::decode(&buf).expect("decode should succeed");
        assert_eq!(1, msg.argc);
        assert!(msg.decode_arg().is_none());
    }

    #[test]
    fn binary_argument_round_trip() {
        let bytes: Vec<u8> = vec![0x00, 0x01, 0xff, 0xfe, 0x80, 0x7f];
        let args: Vec<&[u8]> = vec![&bytes];
        let buf = encode(&args).expect("encode should succeed");
        let mut msg = AmpMessage::decode(&buf).expect("decode should succeed");
        let arg = msg.decode_arg().expect("decode_arg should succeed");
        assert_eq!(bytes.as_slice(), arg.as_slice());
    }

    #[test]
    fn empty_argument_round_trip() {
        let args: Vec<&[u8]> = vec![b""];
        let buf = encode(&args).expect("encode should succeed");
        let mut msg = AmpMessage::decode(&buf).expect("decode should succeed");
        assert_eq!(1, msg.argc);
        let arg = msg.decode_arg().expect("decode_arg should succeed");
        assert!(arg.is_empty());
    }
}
