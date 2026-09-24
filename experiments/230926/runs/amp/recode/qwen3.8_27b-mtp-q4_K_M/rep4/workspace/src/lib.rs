//! AMP (Abstract Message Protocol) — Rust port of the C clibs `amp` package.
//!
//! Wire format:
//!
//! ```text
//! +------------+------------+------------+  (repeated per argument)
//! | <ver/argc> | <length>   | <data>     |
//! | 1 byte     | 4 bytes BE | <len> bytes|
//! +------------+------------+------------+
//! ```
//!
//! Byte 0: high nibble = protocol version (1), low nibble = argument count
//! (0–15). Each argument: 4-byte big-endian length followed by raw bytes.

/// Protocol version (high nibble of byte 0).
///
/// Maps from C `#define AMP_VERSION 1`.
pub const VERSION: u8 = 1;

/// Errors returned by encode/decode operations.
///
/// Replaces C's `NULL` returns (OOM) and undefined behavior on truncated
/// input.
#[derive(Debug, PartialEq)]
pub enum AmpError {
    /// Decode was called on empty input.
    EmptyBuffer,
    /// More than 15 arguments (argc only has a 4-bit field).
    TooManyArgs,
    /// Not enough bytes for the length prefix or the argument data.
    TruncatedArgument,
    /// More `decode_arg` calls than the header's argc.
    ArgMismatch,
}

/// A decoded AMP message with a cursor into the remaining payload.
///
/// Maps from C `amp_t { short version; short argc; char *buf; }`.
/// The cursor (`buf`) is a zero-copy `&'a [u8]` slice that shrinks as
/// arguments are decoded — no per-argument allocation, no `free`.
#[derive(Debug)]
pub struct AmpMessage<'a> {
    /// Protocol version from the header (high nibble of byte 0).
    pub version: u8,
    /// Argument count from the header (low nibble of byte 0).
    pub argc: u8,
    /// Number of arguments not yet decoded.
    remaining: u8,
    /// Cursor: remaining undecoded payload.
    buf: &'a [u8],
}

impl<'a> AmpMessage<'a> {
    /// Decode the header from `buf` (must be non-empty).
    ///
    /// Equivalent of C `amp_decode(amp_t *msg, char *buf)`.
    pub fn new(buf: &'a [u8]) -> Result<Self, AmpError> {
        if buf.is_empty() {
            return Err(AmpError::EmptyBuffer);
        }
        let version = buf[0] >> 4;
        let argc = buf[0] & 0x0f;
        Ok(Self {
            version,
            argc,
            remaining: argc,
            buf: &buf[1..],
        })
    }

    /// Decode the next argument as a slice borrowed from the message buffer.
    ///
    /// Equivalent of C `amp_decode_arg(amp_t *msg)` — but zero-copy: the
    /// returned slice borrows from the original buffer, so no allocation and
    /// no `free` is needed. Advances the cursor.
    pub fn decode_arg(&mut self) -> Result<&'a [u8], AmpError> {
        if self.remaining == 0 {
            return Err(AmpError::ArgMismatch);
        }
        if self.buf.len() < 4 {
            return Err(AmpError::TruncatedArgument);
        }
        let len = u32::from_be_bytes(self.buf[0..4].try_into().unwrap()) as usize;
        if len > self.buf.len() - 4 {
            return Err(AmpError::TruncatedArgument);
        }
        let arg = &self.buf[4..4 + len];
        self.buf = &self.buf[4 + len..];
        self.remaining -= 1;
        Ok(arg)
    }
}

/// Encode an argv into an AMP message buffer.
///
/// Equivalent of C `amp_encode(char **argv, int argc)`.
/// Rejects `argv.len() > 15` with `AmpError::TooManyArgs` (C silently
/// corrupted the header in that case).
pub fn encode(argv: &[&[u8]]) -> Result<Vec<u8>, AmpError> {
    if argv.len() > 15 {
        return Err(AmpError::TooManyArgs);
    }
    let mut out = Vec::with_capacity(1 + argv.iter().map(|a| 4 + a.len()).sum::<usize>());
    out.push((VERSION << 4) | argv.len() as u8);
    for arg in argv {
        out.extend_from_slice(&(arg.len() as u32).to_be_bytes());
        out.extend_from_slice(arg);
    }
    Ok(out)
}
