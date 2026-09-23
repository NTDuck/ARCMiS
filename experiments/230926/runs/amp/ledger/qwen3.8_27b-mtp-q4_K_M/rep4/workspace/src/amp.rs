//! AMP — Abstract Message Protocol.
//!
//! Wire format:
//!
//! ```text
//!          0        1 2 3 4     <length>    ...
//!   +------------+----------+------------+
//!   | <ver/argc> | <length> | <data>     | additional arguments
//!   +------------+----------+------------+
//! ```

use std::fmt;

/// Protocol version.
pub const AMP_VERSION: u8 = 1;

/// Error returned when a buffer is too short or malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmpError {
    /// The buffer was empty; no header byte to decode.
    EmptyBuffer,
    /// Fewer than 4 bytes remained to read the argument length.
    TruncatedLength,
    /// Fewer than the declared number of bytes remained for the payload.
    TruncatedPayload,
}

impl fmt::Display for AmpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AmpError::EmptyBuffer => write!(f, "empty buffer: no header byte"),
            AmpError::TruncatedLength => write!(f, "truncated buffer: missing argument length"),
            AmpError::TruncatedPayload => write!(f, "truncated buffer: missing argument payload"),
        }
    }
}

impl std::error::Error for AmpError {}

/// An AMP message.
///
/// `buf` is a cursor into the caller-owned buffer that shrinks as
/// arguments are decoded.
pub struct Amp<'a> {
    /// Protocol version (upper 4 bits of the header byte).
    pub version: u8,
    /// Number of arguments (lower 4 bits of the header byte).
    pub argc: u8,
    /// Remaining cursor into the message body.
    buf: &'a [u8],
}

impl<'a> Amp<'a> {
    /// Decode the message header in `buf`, positioning the cursor
    /// after the header byte.
    pub fn new(buf: &'a [u8]) -> Result<Self, AmpError> {
        if buf.is_empty() {
            return Err(AmpError::EmptyBuffer);
        }
        Ok(Amp {
            version: buf[0] >> 4,
            argc: buf[0] & 0xf,
            buf: &buf[1..],
        })
    }

    /// Decode the next argument, returning an owned copy of the
    /// payload and advancing the cursor.
    pub fn decode_arg(&mut self) -> Result<Vec<u8>, AmpError> {
        if self.buf.len() < 4 {
            return Err(AmpError::TruncatedLength);
        }
        let len = u32::from_be_bytes(self.buf[0..4].try_into().unwrap()) as usize;
        let rest = &self.buf[4..];
        if rest.len() < len {
            return Err(AmpError::TruncatedPayload);
        }
        let arg = rest[..len].to_vec();
        self.buf = &rest[len..];
        Ok(arg)
    }
}

/// Encode the AMP message `args`.
///
/// The header byte packs the version in the upper 4 bits and the
/// argument count in the lower 4 bits (argc truncated to 4 bits).
pub fn encode(args: &[&[u8]]) -> Vec<u8> {
    let argc = args.len();

    // total length
    let mut len = 1usize;
    for arg in args {
        len += 4 + arg.len();
    }

    let mut buf = Vec::with_capacity(len);

    // ver/argc
    buf.push(AMP_VERSION << 4 | (argc as u8 & 0xf));

    // encode
    for arg in args {
        buf.extend_from_slice(&(arg.len() as u32).to_be_bytes());
        buf.extend_from_slice(arg);
    }

    buf
}
