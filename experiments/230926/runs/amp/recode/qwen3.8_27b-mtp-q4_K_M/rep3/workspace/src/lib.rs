//! AMP — Abstract Message Protocol (Rust translation of clibs/amp).
//!
//! Wire format (must stay byte-identical to the C encoder):
//!
//! ```text
//! +------------+----------+------------+  (repeated per argument)
//! | <ver/argc> | <length> | <data>     |
//! +------------+----------+------------+
//! ```
//!
//! Byte 0: version in the high nibble, argc in the low nibble (argc 0..=15).
//! Each argument: 4-byte big-endian length (u32be) followed by that many raw bytes.

/// Protocol version (high nibble of the header byte). Mirrors C `AMP_VERSION`.
pub const VERSION: u8 = 1;

/// Error type for encode/decode failures. Replaces C `NULL` returns.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// `encode` was asked to encode more than 15 arguments (argc does not fit
    /// the low nibble of the header byte).
    TooManyArguments,
    /// The input buffer is shorter than the header/length fields demand.
    TruncatedBuffer,
}

/// A decoded AMP message. Borrows the original buffer (zero-copy args),
/// mirroring C `amp_t` (whose `buf` field is the cursor).
pub struct Message<'a> {
    /// Protocol version (high nibble of the header byte).
    pub version: u8,
    /// Number of arguments in the message (low nibble of the header byte).
    pub argc: u8,
    /// Cursor into the remaining (argument) bytes, like C `msg->buf`.
    rest: &'a [u8],
    /// How many arguments have been consumed by `arg()` so far.
    consumed: u8,
}

/// Read a u32 from the first 4 bytes of `buf` as big-endian.
/// Mirrors C `read_u32_be` (fixed: uses unsigned `u8` arithmetic).
fn read_u32_be(buf: &[u8]) -> u32 {
    u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]])
}

/// Write `n` as a 4-byte big-endian u32 into `buf` (must have len >= 4).
/// Mirrors C `write_u32_be`.
fn write_u32_be(buf: &mut [u8], n: u32) {
    buf[..4].copy_from_slice(&n.to_be_bytes());
}

/// Encode an argv into an AMP buffer. Mirrors C `amp_encode`.
///
/// Returns an owned `Vec<u8>` (RAII replaces malloc/free).
/// Fails with `Error::TooManyArguments` if `args.len() > 15`.
pub fn encode<I, T>(args: I) -> Result<Vec<u8>, Error>
where
    I: IntoIterator<Item = T>,
    T: AsRef<[u8]>,
{
    let args: Vec<T> = args.into_iter().collect();
    let argc = args.len();
    if argc > 15 {
        return Err(Error::TooManyArguments);
    }
    let argc = argc as u8;

    // Total size = 1 (header) + sum(4 + len(arg)).
    let mut total = 1usize;
    for a in &args {
        total += 4 + a.as_ref().len();
    }

    let mut out = Vec::with_capacity(total);
    out.push((VERSION << 4) | argc);
    for a in &args {
        let bytes = a.as_ref();
        let mut len = [0u8; 4];
        write_u32_be(&mut len, bytes.len() as u32);
        out.extend_from_slice(&len);
        out.extend_from_slice(bytes);
    }
    Ok(out)
}

/// Decode the header of `buf` into a `Message` that borrows `buf`.
/// Mirrors C `amp_decode`. Fails with `Error::TruncatedBuffer` if `buf` is empty.
pub fn decode<'a>(buf: &'a [u8]) -> Result<Message<'a>, Error> {
    if buf.is_empty() {
        return Err(Error::TruncatedBuffer);
    }
    let version = buf[0] >> 4;
    let argc = buf[0] & 0xf;
    Ok(Message {
        version,
        argc,
        rest: &buf[1..],
        consumed: 0,
    })
}

impl<'a> Message<'a> {
    /// Decode the next argument (zero-copy slice) and advance the cursor.
    /// Mirrors C `amp_decode_arg` (but borrows instead of malloc'ing a copy).
    ///
    /// Returns `None` once all `argc` arguments have been consumed or the
    /// buffer is truncated.
    pub fn arg(&mut self) -> Option<&'a [u8]> {
        if self.consumed >= self.argc {
            return None;
        }
        // Need at least 4 bytes for the length field.
        if self.rest.len() < 4 {
            return None;
        }
        let n = read_u32_be(&self.rest[..4]) as usize;
        // Need `n` more bytes after the length field.
        if self.rest.len() < 4 + n {
            return None;
        }
        // Copy the cursor reference (it is `Copy`) so the returned slice and
        // the new cursor both derive from a local, avoiding a self-borrow.
        let rest = self.rest;
        let slice = &rest[4..4 + n];
        self.rest = &rest[4 + n..];
        self.consumed += 1;
        Some(slice)
    }
}
