//! AMP (Abstract Message Protocol) — Rust translation of the C `amp` crate.
//!
//! Wire format (must stay byte-identical to the C implementation):
//!
//! ```text
//!         0        1 2 3 4     <length>    ...
//!   +------------+----------+------------+
//!   | <ver/argc> | <length> | <data>     | additional arguments
//!   +------------+----------+------------+
//! ```
//!
//! One header byte (`version` in the high nibble, `argc` in the low nibble),
//! then for each argument a big-endian u32 length followed by that many raw
//! bytes.

/// Protocol version (high nibble of the header byte).
///
/// C: `#define AMP_VERSION 1`
pub const VERSION: u8 = 1;

/// Error type for [`encode`].
///
/// C's `amp_encode` silently masks `argc` into 4 bits; the Rust API
/// surfaces the protocol limit (argc > 15) as an error instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmpError {
    /// More than 15 arguments: the protocol's 4-bit argc field cannot
    /// represent the count.
    ArgcTooLarge { argc: usize },
}

impl std::fmt::Display for AmpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AmpError::ArgcTooLarge { argc } => write!(
                f,
                "argc {argc} exceeds the protocol limit of 15 (4-bit field)"
            ),
        }
    }
}

impl std::error::Error for AmpError {}

/// A decoded AMP message with a cursor over the remaining argument bytes.
///
/// C: `typedef struct { short version; short argc; char *buf; } amp_t;`
///
/// The `cursor` field replaces C's mutable `amp_t.buf` pointer: it borrows
/// the input buffer (zero-copy) and is advanced by [`Message::decode_arg`].
pub struct Message<'a> {
    /// Protocol version from the header byte (high nibble).
    pub version: u8,
    /// Argument count from the header byte (low nibble).
    pub argc: u8,
    /// Cursor over the not-yet-decoded argument bytes.
    cursor: &'a [u8],
}

impl<'a> Message<'a> {
    /// Decode the header from `buf` (must be non-empty).
    ///
    /// C: `void amp_decode(amp_t *msg, char *buf);`
    ///
    /// Sets `version`/`argc` from the first byte and positions the cursor
    /// at the first argument byte.
    ///
    /// # Panics
    ///
    /// Panics if `buf` is empty. C's `amp_decode` reads `buf[0]`
    /// unconditionally, so an empty buffer is undefined behavior there; we
    /// make that a clear, explicit panic instead.
    pub fn decode(buf: &'a [u8]) -> Self {
        let header = *buf
            .first()
            .expect("amp: cannot decode an empty buffer (need at least the header byte)");
        let version = header >> 4;
        let argc = header & 0xf;
        let cursor = &buf[1..];
        Message {
            version,
            argc,
            cursor,
        }
    }

    /// Decode the next argument, advancing the cursor.
    ///
    /// C: `char *amp_decode_arg(amp_t *msg);` (malloc'd copy, NULL on OOM)
    ///
    /// Reads a big-endian u32 length, then that many bytes. Returns `None`
    /// if the buffer is exhausted or the declared length overflows the
    /// remaining bytes (C's NULL-return analogue; the real failure mode is
    /// truncation, not OOM).
    pub fn decode_arg(&mut self) -> Option<&'a [u8]> {
        // Need at least 4 bytes for the big-endian u32 length.
        if self.cursor.len() < 4 {
            return None;
        }
        let len = u32::from_be_bytes(self.cursor[..4].try_into().unwrap()) as usize;
        let rest = &self.cursor[4..];
        if len > rest.len() {
            return None;
        }
        let arg = &rest[..len];
        self.cursor = &rest[len..];
        Some(arg)
    }
}

/// Encode an argv into an AMP message buffer.
///
/// C: `char *amp_encode(char **argv, int argc);` (caller frees)
///
/// Returns the owned wire buffer. `Err(AmpError::ArgcTooLarge)` when
/// `argv.len() > 15`; the happy path (argc <= 15) is byte-identical to C.
pub fn encode(argv: &[&str]) -> Result<Vec<u8>, AmpError> {
    if argv.len() > 15 {
        return Err(AmpError::ArgcTooLarge { argc: argv.len() });
    }

    let argc = argv.len() as u8;
    let header = (VERSION << 4) | argc;

    let total: usize = argv.iter().map(|a| 4 + a.len()).sum::<usize>() + 1;
    let mut out = Vec::with_capacity(total);
    out.push(header);
    for arg in argv {
        let bytes = arg.as_bytes();
        out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        out.extend_from_slice(bytes);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip with an empty argv: encode of `&[]` must be the single
    /// header byte `0x10` (version 1, argc 0).
    #[test]
    fn encode_empty_argv() {
        assert_eq!(encode(&[]), Ok(vec![0x10]));
    }

    /// Empty-string arguments (length 0) must round-trip.
    #[test]
    fn round_trip_empty_string_arg() {
        let buf = encode(&[""]).expect("encode should succeed");
        let mut msg = Message::decode(&buf);
        assert_eq!(msg.argc, 1);
        let arg = msg.decode_arg().expect("one arg should decode");
        assert_eq!(arg, b"");
        assert!(msg.decode_arg().is_none());
    }

    /// A truncated buffer must yield `None` from `decode_arg` instead of
    /// panicking.
    #[test]
    fn decode_arg_truncated_returns_none() {
        // Header (argc 1) + a declared length of 5, but only 2 data bytes
        // actually present.
        let buf: Vec<u8> = vec![0x11, 0x00, 0x00, 0x00, 0x05, b'a', b'b'];
        let mut msg = Message::decode(&buf);
        assert_eq!(msg.argc, 1);
        assert!(msg.decode_arg().is_none());
    }
}
