//
// amp
//
// Copyright (c) 2014 TJ Holowaychuk <tj@vision-media.ca>
//

//! C implementation of the uber simple
//! [AMP](https://github.com/visionmedia/node-amp) protocol.

/// Protocol version.
pub const AMP_VERSION: u8 = 1;

/// Message struct.
#[derive(Debug, Default, Clone, Copy)]
pub struct AmpMessage<'a> {
    pub version: u16,
    pub argc: u16,
    pub buf: &'a [u8],
}

/// Read u32be.
fn read_u32_be(buf: &[u8]) -> u32 {
    u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]])
}

/// Decode the `msg` header in `buf`.
pub fn amp_decode<'a>(msg: &mut AmpMessage<'a>, buf: &'a [u8]) {
    msg.version = (buf[0] >> 4) as u16;
    msg.argc = (buf[0] & 0xf) as u16;
    msg.buf = &buf[1..];
}

/// Decode `msg` argument, returning a buffer
/// and progressing the msg.buf cursor.
pub fn amp_decode_arg(msg: &mut AmpMessage) -> Vec<u8> {
    let len = read_u32_be(msg.buf) as usize;
    msg.buf = &msg.buf[4..];

    let buf = msg.buf[..len].to_vec();
    msg.buf = &msg.buf[len..];
    buf
}

/// Encode the AMP message argv.
///
/// ```text
///         0        1 2 3 4     <length>    ...
///   +------------+----------+------------+
///   | <ver/argc> | <length> | <data>     | additional arguments
///   +------------+----------+------------+
/// ```
pub fn amp_encode(argv: &[&str]) -> Vec<u8> {
    let argc = argv.len();

    // length
    let mut len = 1usize;
    let mut lens = Vec::with_capacity(argc);
    for arg in argv {
        let l = arg.len();
        len += 4;
        lens.push(l);
        len += l;
    }

    // alloc
    let mut buf = Vec::with_capacity(len);

    // ver/argc
    buf.push((AMP_VERSION << 4) | argc as u8);

    // encode
    for (arg, l) in argv.iter().zip(lens.iter()) {
        buf.extend_from_slice(&(*l as u32).to_be_bytes());
        buf.extend_from_slice(arg.as_bytes());
    }

    buf
}
