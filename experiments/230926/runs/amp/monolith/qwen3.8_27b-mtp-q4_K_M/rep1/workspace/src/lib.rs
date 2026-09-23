//!
//! amp.rs
//!
//! Copyright (c) 2014 TJ Holowaychuk <tj@vision-media.ca>
//!
//! C implementation of the uber simple AMP protocol, translated to Rust.
//!

/// Protocol version.
pub const AMP_VERSION: u8 = 1;

/// Message struct.
///
/// `buf` is the remaining (not yet decoded) portion of the encoded
/// message; decoding arguments progresses the cursor.
#[derive(Debug, Default, Clone, Copy)]
pub struct Amp {
    pub version: u16,
    pub argc: u16,
    pub buf: *const u8,
    pub len: usize,
}

impl Amp {
    /// Returns the remaining undecoded bytes as a slice.
    #[inline]
    fn rest(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.buf, self.len) }
    }

    /// Advances the cursor by `n` bytes.
    #[inline]
    fn advance(&mut self, n: usize) {
        self.buf = unsafe { self.buf.add(n) };
        self.len -= n;
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

/// Decode the `msg` header in `buf`.
pub fn amp_decode(msg: &mut Amp, buf: &[u8]) {
    msg.version = (buf[0] >> 4) as u16;
    msg.argc = (buf[0] & 0xf) as u16;
    msg.buf = unsafe { buf.as_ptr().add(1) };
    msg.len = buf.len() - 1;
}

/// Decode `msg` argument, returning a buffer
/// and progressing the msg->buf cursor.
pub fn amp_decode_arg(msg: &mut Amp) -> Option<Vec<u8>> {
    let rest = msg.rest();
    if rest.len() < 4 {
        return None;
    }
    let len = read_u32_be(rest) as usize;
    msg.advance(4);

    let rest = msg.rest();
    if rest.len() < len {
        return None;
    }
    let out = rest[..len].to_vec();
    msg.advance(len);
    Some(out)
}

/// Encode the AMP message argv.
///
/// ```text
///         0        1 2 3 4     <length>    ...
///   +------------+----------+------------+
///   | <ver/argc> | <length> | <data>     | additional arguments
///   +------------+----------+------------+
/// ```
///
pub fn amp_encode(argv: &[&str]) -> Vec<u8> {
    let argc = argv.len();
    let mut len = 1usize;

    // length
    let mut lens = Vec::with_capacity(argc);
    for arg in argv {
        len += 4;
        let l = arg.len();
        lens.push(l);
        len += l;
    }

    // alloc
    let mut buf = vec![0u8; len];

    // ver/argc
    buf[0] = AMP_VERSION << 4 | argc as u8;

    // encode
    let mut off = 1;
    for (i, arg) in argv.iter().enumerate() {
        let l = lens[i];

        write_u32_be(&mut buf[off..off + 4], l as u32);
        off += 4;

        buf[off..off + l].copy_from_slice(arg.as_bytes());
        off += l;
    }

    buf
}
