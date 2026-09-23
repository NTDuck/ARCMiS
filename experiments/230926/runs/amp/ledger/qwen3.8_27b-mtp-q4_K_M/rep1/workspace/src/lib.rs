//! Rust translation of the C AMP (Argument Message Protocol) library.
//!
//! Wire format:
//! - 1 header byte: `version << 4 | argc`
//! - for each argument: 4-byte big-endian length followed by that many bytes

/// Protocol version, matching `AMP_VERSION` in the C header.
pub const AMP_VERSION: u8 = 1;

/// A decoded AMP message with a cursor over the argument payload.
pub struct AmpMsg<'a> {
    pub version: u8,
    pub argc: u8,
    buf: &'a [u8],
    pos: usize,
}

/// Encode `argv` into the AMP wire format.
///
/// Header byte is `AMP_VERSION << 4 | argc`; each argument is a 4-byte
/// big-endian length followed by its bytes.
pub fn amp_encode(argv: &[&str]) -> Vec<u8> {
    let argc = argv.len() as u8;
    let mut out = Vec::with_capacity(1 + argv.iter().map(|a| 4 + a.len()).sum::<usize>());
    out.push(AMP_VERSION << 4 | argc);
    for arg in argv {
        let bytes = arg.as_bytes();
        out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        out.extend_from_slice(bytes);
    }
    out
}

/// Decode the header of an AMP message. The cursor starts just past the
/// header byte, ready for `AmpMsg::amp_decode_arg`.
pub fn amp_decode(buf: &[u8]) -> AmpMsg<'_> {
    let version = buf[0] >> 4;
    let argc = buf[0] & 0xf;
    AmpMsg {
        version,
        argc,
        buf: &buf[1..],
        pos: 0,
    }
}

impl<'a> AmpMsg<'a> {
    /// Decode the next argument: read a 4-byte big-endian length, copy that
    /// many bytes, and advance the cursor.
    pub fn amp_decode_arg(&mut self) -> Vec<u8> {
        let len = u32::from_be_bytes([
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
        ]) as usize;
        self.pos += 4;
        let arg = self.buf[self.pos..self.pos + len].to_vec();
        self.pos += len;
        arg
    }
}
