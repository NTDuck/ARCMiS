//!
//! amp — Abstract Message Protocol (Rust translation of the C library).
//!
//! The protocol is the uber-simple **argv protocol**:
//!
//! ```text
//!          0        1 2 3 4     <length>    ...
//!  +------------+----------+------------+
//!  | <ver/argc> | <length> | <data>     | additional arguments
//!  +------------+----------+------------+
//! ```
//!
//! - Byte 0: header — high nibble = version (`AMP_VERSION`),
//!   low nibble = argc (0..15).
//! - Then, per argument: 4-byte **big-endian** length, followed by that
//!   many raw bytes.

/// Protocol version.
pub const AMP_VERSION: u8 = 1;

/// Decoded AMP message with a read cursor over the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amp {
    /// Protocol version (high nibble of the header byte).
    pub version: u8,
    /// Number of arguments (low nibble of the header byte).
    pub argc: u8,
    /// Payload after the header byte.
    pub buf: Vec<u8>,
    /// Cursor advanced by [`amp_decode_arg`].
    pub pos: usize,
}

impl Default for Amp {
    fn default() -> Self {
        Self {
            version: 0,
            argc: 0,
            buf: Vec::new(),
            pos: 0,
        }
    }
}

impl Amp {
    /// Construct a zeroed message (all fields zero/empty).
    pub fn new() -> Self {
        Self::default()
    }
}

/// Read u32be.
fn read_u32_be(buf: &[u8]) -> u32 {
    u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]])
}

/// Write u32be.
fn write_u32_be(buf: &mut [u8], n: u32) {
    buf[0] = (n >> 24) as u8;
    buf[1] = (n >> 16) as u8;
    buf[2] = (n >> 8) as u8;
    buf[3] = n as u8;
}

/// Decode the `msg` header in `buf`.
///
/// `msg.version = buf[0] >> 4`, `msg.argc = buf[0] & 0xf`, and the payload
/// becomes `buf[1..]` with the cursor reset to 0.
pub fn amp_decode(msg: &mut Amp, buf: &[u8]) {
    msg.version = buf[0] >> 4;
    msg.argc = buf[0] & 0xf;
    msg.buf = buf[1..].to_vec();
    msg.pos = 0;
}

/// Decode `msg` argument, returning a fresh buffer and progressing
/// the `msg.pos` cursor.
///
/// Reads a u32be length at `msg.pos`, advances the cursor by 4, copies
/// `len` bytes, and advances the cursor by `len`.
pub fn amp_decode_arg(msg: &mut Amp) -> Vec<u8> {
    let len = read_u32_be(&msg.buf[msg.pos..]) as usize;
    msg.pos += 4;

    let arg = msg.buf[msg.pos..msg.pos + len].to_vec();
    msg.pos += len;
    arg
}

/// Encode the AMP message argv.
///
/// ```text
///          0        1 2 3 4     <length>    ...
///  +------------+----------+------------+
///  | <ver/argc> | <length> | <data>     | additional arguments
///  +------------+----------+------------+
/// ```
///
/// Header byte is `AMP_VERSION << 4 | argc`, then per arg: 4-byte
/// big-endian length + raw bytes.
pub fn amp_encode(argv: &[&[u8]]) -> Vec<u8> {
    let argc = argv.len();

    // length
    let mut len: usize = 1;
    for arg in argv {
        len += 4;
        len += arg.len();
    }

    // alloc
    let mut buf = vec![0u8; len];
    let mut pos = 0;

    // ver/argc
    buf[pos] = AMP_VERSION << 4 | argc as u8;
    pos += 1;

    // encode
    for arg in argv {
        write_u32_be(&mut buf[pos..pos + 4], arg.len() as u32);
        pos += 4;

        buf[pos..pos + arg.len()].copy_from_slice(arg);
        pos += arg.len();
    }

    buf
}
