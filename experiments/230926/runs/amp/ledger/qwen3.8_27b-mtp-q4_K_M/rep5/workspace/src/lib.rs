//
// amp.rs
//
// Copyright (c) 2014 TJ Holowaychuk <tj@vision-media.ca>
//
// Rust port of amp.c / amp.h.
//

/// Protocol version.
pub const AMP_VERSION: u8 = 1;

/// Message struct.
///
/// `buf` holds the payload after the header byte; `pos` is the
/// cursor into `buf` (replaces C's `char *buf` pointer advancement).
#[derive(Debug, Default)]
pub struct Amp {
    pub version: u8,
    pub argc: u8,
    pub buf: Vec<u8>,
    pub pos: usize,
}

/// Decode the `msg` header in `buf`.
pub fn amp_decode(msg: &mut Amp, buf: &[u8]) {
    msg.version = buf[0] >> 4;
    msg.argc = buf[0] & 0xf;
    msg.buf = buf[1..].to_vec();
    msg.pos = 0;
}

/// Decode `msg` argument, returning a buffer and progressing
/// the `msg.pos` cursor.
pub fn amp_decode_arg(msg: &mut Amp) -> Vec<u8> {
    let len = u32::from_be_bytes(msg.buf[msg.pos..msg.pos + 4].try_into().unwrap()) as usize;
    msg.pos += 4;

    let out = msg.buf[msg.pos..msg.pos + len].to_vec();
    msg.pos += len;
    out
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
    let mut len: usize = 1;

    // length
    let lens: Vec<usize> = argv.iter().map(|a| a.len()).collect();
    for i in 0..argc {
        len += 4;
        len += lens[i];
    }

    // alloc
    let mut buf = vec![0u8; len];
    let mut pos = 0;

    // ver/argc
    buf[pos] = AMP_VERSION << 4 | argc as u8;
    pos += 1;

    // encode
    for i in 0..argc {
        let len = lens[i];

        buf[pos..pos + 4].copy_from_slice(&(len as u32).to_be_bytes());
        pos += 4;

        buf[pos..pos + len].copy_from_slice(argv[i].as_bytes());
        pos += len;
    }

    buf
}
