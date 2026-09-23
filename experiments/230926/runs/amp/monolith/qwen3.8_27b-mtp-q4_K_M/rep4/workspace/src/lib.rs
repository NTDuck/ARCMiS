//
// amp
//
// Copyright (c) 2014 TJ Holowaychuk <tj@vision-media.ca>
//

//! C implementation of the uber simple [AMP](https://github.com/visionmedia/node-amp) protocol.

/// Protocol version.
pub const AMP_VERSION: u8 = 1;

/// Message struct.
#[derive(Debug, Default)]
pub struct Amp<'a> {
    pub version: u8,
    pub argc: u8,
    pub buf: &'a [u8],
}

/// Read u32be.
fn read_u32_be(buf: &[u8]) -> u32 {
    let n = 0u32;
    n | (buf[0] as u32) << 24
        | (buf[1] as u32) << 16
        | (buf[2] as u32) << 8
        | buf[3] as u32
}

/// Write u32be.
fn write_u32_be(buf: &mut [u8], n: u32) {
    buf[0] = (n >> 24) as u8 & 0xff;
    buf[1] = (n >> 16) as u8 & 0xff;
    buf[2] = (n >> 8) as u8 & 0xff;
    buf[3] = n as u8 & 0xff;
}

/// Decode the `msg` header in `buf`.
pub fn amp_decode<'a>(msg: &mut Amp<'a>, buf: &'a [u8]) {
    msg.version = buf[0] >> 4;
    msg.argc = buf[0] & 0xf;
    msg.buf = &buf[1..];
}

/// Decode `msg` argument, returning a buffer
/// that must be freed by the user and progressing
/// the msg.buf cursor.
pub fn amp_decode_arg(msg: &mut Amp) -> Vec<u8> {
    let len = read_u32_be(msg.buf) as usize;
    msg.buf = &msg.buf[4..];

    let mut buf = Vec::with_capacity(len);
    buf.extend_from_slice(&msg.buf[..len]);
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

        let mut be = [0u8; 4];
        write_u32_be(&mut be, l as u32);
        buf[off..off + 4].copy_from_slice(&be);
        off += 4;

        buf[off..off + l].copy_from_slice(arg.as_bytes());
        off += l;
    }

    buf
}
