//! AMP — Abstract Message Protocol (Rust translation of clibs/amp).
//!
//! Wire format (preserved byte-for-byte from the C implementation):
//!
//! ```text
//! +------------+----------+------------+----------+------------+
//! | <ver/argc> | <length> | <data>     | <length> | <data>     | ...
//! +------------+----------+------------+----------+------------+
//!    1 byte        4 bytes (u32 BE)   len bytes
//! ```
//!
//! Byte 0 is `version << 4 | argc`; `argc` is limited to 4 bits (max 15 args).

/// Protocol version. Mirrors C `AMP_VERSION`.
pub const VERSION: u8 = 1;

/// A decoded AMP message with a cursor into the payload.
/// Mirrors C `amp_t`.
pub struct Message<'a> {
    /// Protocol version (high nibble of the header byte).
    pub version: u8,
    /// Number of arguments (low nibble of the header byte, 0..=15).
    pub argc: u8,
    /// Remaining payload; advanced by [`Message::decode_arg`].
    buf: &'a [u8],
}

impl<'a> Message<'a> {
    /// Decode the 1-byte header from `buf`.
    /// Mirrors C `amp_decode(amp_t *msg, char *buf)`.
    pub fn decode(buf: &'a [u8]) -> Self {
        let header = buf[0];
        Self {
            version: header >> 4,
            argc: header & 0xf,
            buf: &buf[1..],
        }
    }

    /// Decode the next argument, advancing the cursor.
    /// Mirrors C `amp_decode_arg(amp_t *msg)`.
    ///
    /// Returns `None` if the buffer is truncated (C read out of bounds — UB).
    pub fn decode_arg(&mut self) -> Option<String> {
        // Need at least 4 bytes for the big-endian length.
        let len = u32::from_be_bytes(self.buf.get(..4)?.try_into().ok()?) as usize;
        // Need `len` bytes remaining for the payload.
        let data = self.buf.get(4..4 + len)?;
        let s = String::from_utf8_lossy(data).into_owned();
        self.buf = &self.buf[4 + len..];
        Some(s)
    }
}

/// Encode an argv slice into the AMP wire format.
/// Mirrors C `amp_encode(char **argv, int argc)`.
///
/// Returns `None` if `argv.len() > 15` (4-bit argc field) — the C version
/// silently corrupted the header; here the error is explicit.
pub fn encode(argv: &[&str]) -> Option<Vec<u8>> {
    let argc = argv.len() as u8;
    if argc > 15 {
        return None;
    }
    let mut out = Vec::with_capacity(1 + argv.iter().map(|a| 4 + a.len()).sum::<usize>());
    out.push(VERSION << 4 | argc);
    for arg in argv {
        out.extend_from_slice(&(arg.len() as u32).to_be_bytes());
        out.extend_from_slice(arg.as_bytes());
    }
    Some(out)
}
