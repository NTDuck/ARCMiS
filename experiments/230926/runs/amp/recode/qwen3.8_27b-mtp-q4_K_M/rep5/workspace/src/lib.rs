//! AMP — Abstract Message Protocol.
//!
//! Rust port of the C library `clibs/amp` (TJ Holowaychuk, MIT).
//!
//! Wire format:
//!
//! ```text
//! +------------+----------+------------+ ...
//! | <ver/argc> | <length> | <data>     | additional arguments
//! +------------+----------+------------+
//! ```
//!
//! - Byte 0: protocol version in the high nibble, argc in the low nibble.
//! - Each argument: 4-byte big-endian length, then that many raw bytes.
//! - Arguments are length-prefixed binary (may contain NUL bytes),
//!   never NUL-terminated.

/// Protocol version (high nibble of the header byte).
///
/// C: `#define AMP_VERSION 1`
pub const VERSION: u8 = 1;

/// Errors returned by `decode` / `decode_arg`.
///
/// C signals these failures with `NULL` returns; Rust threads them
/// through `Result`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmpError {
    /// The buffer is shorter than the 1-byte header.
    Truncated,
    /// An argument length field exceeds the remaining buffer.
    BadLength,
    /// More than 15 arguments: argc does not fit the 4-bit header field.
    TooManyArgs,
}

impl std::fmt::Display for AmpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AmpError::Truncated => write!(f, "truncated AMP buffer"),
            AmpError::BadLength => write!(f, "argument length exceeds remaining buffer"),
            AmpError::TooManyArgs => write!(f, "argc exceeds the 4-bit limit (15)"),
        }
    }
}

impl std::error::Error for AmpError {}

/// Decoded AMP message header with a cursor into the payload.
///
/// C: `typedef struct { short version; short argc; char *buf; } amp_t;`
///
/// The `buf` field is a borrowed slice (not an owned copy), so
/// `decode_arg` can return a zero-copy `&[u8]` borrowing from the
/// message instead of the C per-argument `malloc`.
#[derive(Default)]
pub struct AmpMessage<'a> {
    /// Protocol version from the header (high nibble).
    pub version: u8,
    /// Argument count from the header (low nibble).
    pub argc: u8,
    /// Remaining payload; advanced by `decode_arg`.
    buf: &'a [u8],
}


/// Encode an argv into an AMP message buffer.
///
/// C: `char *amp_encode(char **argv, int argc);`
///
/// Returns the owned wire buffer (C returned a `malloc`'d buffer the
/// caller had to free). Panics if `argv.len() > 15`, since argc does
/// not fit the 4-bit header field (the C code silently corrupts the
/// header in that case).
pub fn encode(argv: &[&[u8]]) -> Vec<u8> {
    let argc = argv.len();
    // argc is packed into the low 4 bits of the header byte; the C code
    // silently corrupts the header for argc >= 16, so we refuse to do so.
    assert!(argc <= 15, "argc {} exceeds the 4-bit limit (15)", argc);

    let mut out = Vec::with_capacity(1 + argv.iter().map(|a| 4 + a.len()).sum::<usize>());
    out.push((VERSION << 4) | (argc as u8));
    for arg in argv {
        out.extend_from_slice(&(arg.len() as u32).to_be_bytes());
        out.extend_from_slice(arg);
    }
    out
}

/// Parse the 1-byte header of `buf` into `msg`, pointing the cursor at
/// the payload.
///
/// C: `void amp_decode(amp_t *msg, char *buf);`
///
/// Returns `Err(AmpError::Truncated)` if `buf` is shorter than 1 byte.
pub fn decode<'a>(msg: &mut AmpMessage<'a>, buf: &'a [u8]) -> Result<(), AmpError> {
    if buf.is_empty() {
        return Err(AmpError::Truncated);
    }
    msg.version = buf[0] >> 4;
    msg.argc = buf[0] & 0xf;
    msg.buf = &buf[1..];
    Ok(())
}

/// Decode the next argument, advancing the cursor.
///
/// C: `char *amp_decode_arg(amp_t *msg);`
///
/// Zero-copy: the returned slice borrows from `msg` (C returned a
/// `malloc`'d copy the caller had to free). Returns
/// `Err(AmpError::BadLength)` if the 4-byte length field or the
/// argument data is missing from the remaining buffer.
pub fn decode_arg<'a>(msg: &mut AmpMessage<'a>) -> Result<&'a [u8], AmpError> {
    // Need at least 4 bytes for the big-endian length field.
    if msg.buf.len() < 4 {
        return Err(AmpError::BadLength);
    }
    let len = u32::from_be_bytes([msg.buf[0], msg.buf[1], msg.buf[2], msg.buf[3]]) as usize;
    // The length must not exceed the remaining payload (no wrapping).
    if msg.buf.len() - 4 < len {
        return Err(AmpError::BadLength);
    }
    let arg = &msg.buf[4..4 + len];
    msg.buf = &msg.buf[4 + len..];
    Ok(arg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_argv() {
        let buf = encode(&[]);
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], (VERSION << 4) | 0);

        let mut msg = AmpMessage::default();
        decode(&mut msg, &buf).unwrap();
        assert_eq!(msg.version, VERSION);
        assert_eq!(msg.argc, 0);
        assert_eq!(decode_arg(&mut msg), Err(AmpError::BadLength));
    }

    #[test]
    fn single_arg() {
        let buf = encode(&[b"hello"]);
        let mut msg = AmpMessage::default();
        decode(&mut msg, &buf).unwrap();
        assert_eq!(msg.version, VERSION);
        assert_eq!(msg.argc, 1);
        assert_eq!(decode_arg(&mut msg).unwrap(), b"hello");
    }

    #[test]
    fn binary_arg_with_nul() {
        let arg: Vec<u8> = vec![0x00, 0x01, 0x02, 0x00, 0xff];
        let buf = encode(&[&arg]);
        let mut msg = AmpMessage::default();
        decode(&mut msg, &buf).unwrap();
        assert_eq!(msg.argc, 1);
        assert_eq!(decode_arg(&mut msg).unwrap(), arg.as_slice());
    }

    #[test]
    fn truncated_buffer_errors() {
        // decode on an empty buffer -> Truncated
        let mut msg = AmpMessage::default();
        assert_eq!(decode(&mut msg, &[]), Err(AmpError::Truncated));

        // decode_arg past the end -> BadLength
        let buf = encode(&[b"hello"]);
        let mut msg = AmpMessage::default();
        decode(&mut msg, &buf).unwrap();
        assert_eq!(decode_arg(&mut msg).unwrap(), b"hello");
        assert_eq!(decode_arg(&mut msg), Err(AmpError::BadLength));
    }

    #[test]
    #[should_panic]
    fn too_many_args_rejected() {
        let owned: Vec<[u8; 1]> = (0..16).map(|i| [i as u8]).collect();
        let args: Vec<&[u8]> = owned.iter().map(|a| a.as_slice()).collect();
        encode(&args);
    }
}
