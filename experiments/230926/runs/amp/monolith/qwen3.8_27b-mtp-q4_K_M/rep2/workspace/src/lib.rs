//!
//! amp.rs
//!
//! Copyright (c) 2014 TJ Holowaychuk <tj@vision-media.ca>
//!
//! Rust implementation of the uber simple AMP protocol.
//!

/// Protocol version.
pub const AMP_VERSION: u8 = 1;

/// Message struct.
#[derive(Debug, Default)]
pub struct Amp {
    pub version: u16,
    pub argc: u16,
    buf: Vec<u8>,
    pos: usize,
}

impl Amp {
    /// Decode the `msg` header in `buf`.
    pub fn decode(buf: &[u8]) -> Amp {
        let version = (buf[0] >> 4) as u16;
        let argc = (buf[0] & 0xf) as u16;
        Amp {
            version,
            argc,
            buf: buf[1..].to_vec(),
            pos: 0,
        }
    }

    /// Decode `msg` argument, returning a buffer
    /// and progressing the cursor.
    pub fn decode_arg(&mut self) -> Option<Vec<u8>> {
        let len = read_u32_be(&self.buf[self.pos..self.pos + 4]) as usize;
        self.pos += 4;

        let arg = self.buf[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Some(arg)
    }
}

/// Read u32be.
fn read_u32_be(buf: &[u8]) -> u32 {
    let mut n: u32 = 0;
    n |= (buf[0] as u32) << 24;
    n |= (buf[1] as u32) << 16;
    n |= (buf[2] as u32) << 8;
    n |= buf[3] as u32;
    n
}

/// Write u32be.
fn write_u32_be(buf: &mut [u8], n: u32) {
    buf[0] = ((n >> 24) & 0xff) as u8;
    buf[1] = ((n >> 16) & 0xff) as u8;
    buf[2] = ((n >> 8) & 0xff) as u8;
    buf[3] = (n & 0xff) as u8;
}

/// Encode the AMP message argv.
///
/// ```text
///         0        1 2 3 4     <length>    ...
///   +------------+----------+------------+
///   | <ver/argc> | <length> | <data>     | additional arguments
///   +------------+----------+------------+
/// ```
pub fn encode(argv: &[&[u8]]) -> Vec<u8> {
    let argc = argv.len();

    // length
    let mut len: usize = 1;
    let mut lens = Vec::with_capacity(argc);
    for arg in argv {
        len += 4;
        lens.push(arg.len());
        len += arg.len();
    }

    // alloc
    let mut buf = vec![0u8; len];

    // ver/argc
    buf[0] = AMP_VERSION << 4 | argc as u8;

    // encode
    let mut pos = 1;
    for (i, arg) in argv.iter().enumerate() {
        let len = lens[i];

        write_u32_be(&mut buf[pos..pos + 4], len as u32);
        pos += 4;

        buf[pos..pos + len].copy_from_slice(arg);
        pos += len;
    }

    buf
}
