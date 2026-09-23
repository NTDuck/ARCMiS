//!
//! amp
//!
//! Rust implementation of the uber simple AMP protocol.
//!

/// Protocol version.
pub const AMP_VERSION: u8 = 1;

/// Message struct.
#[derive(Debug, Default, Clone)]
pub struct AmpMsg {
    pub version: u16,
    pub argc: u16,
    /// Remaining encoded payload (after the header byte).
    pub buf: Vec<u8>,
    /// Cursor into `buf`.
    pub pos: usize,
}

/// Read u32be.
fn read_u32_be(buf: &[u8]) -> u32 {
    u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]])
}

/// Decode the `msg` header in `buf`.
pub fn amp_decode(msg: &mut AmpMsg, buf: &[u8]) {
    msg.version = (buf[0] >> 4) as u16;
    msg.argc = (buf[0] & 0xf) as u16;
    msg.buf = buf[1..].to_vec();
    msg.pos = 0;
}

/// Decode `msg` argument, returning the argument bytes
/// and progressing the msg cursor.
pub fn amp_decode_arg(msg: &mut AmpMsg) -> Vec<u8> {
    let len = read_u32_be(&msg.buf[msg.pos..msg.pos + 4]) as usize;
    msg.pos += 4;

    let out = msg.buf[msg.pos..msg.pos + len].to_vec();
    msg.pos += len;
    out
}

/// Encode the AMP message argv.
///
/// Layout: a single version/argc byte, followed by each argument as a
/// big-endian u32 length and the argument data, with additional
/// arguments appended after.
///
pub fn amp_encode(argv: &[&str]) -> Vec<u8> {
    let argc = argv.len() as u8;

    // length
    let mut len: usize = 1;
    for arg in argv {
        len += 4;
        len += arg.len();
    }

    // alloc
    let mut buf = Vec::with_capacity(len);

    // ver/argc
    buf.push((AMP_VERSION << 4) | argc);

    // encode
    for arg in argv {
        let bytes = arg.as_bytes();
        buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        buf.extend_from_slice(bytes);
    }

    buf
}
