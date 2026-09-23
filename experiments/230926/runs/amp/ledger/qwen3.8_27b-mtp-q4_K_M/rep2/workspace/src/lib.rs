//! Rust port of the `amp` protocol library (C `src/amp.c` / `src/amp.h`).
//!
//! Wire format (byte-compatible with the C implementation):
//! - Byte 0: `(version << 4) | argc` (version in the high nibble, argc in the
//!   low nibble).
//! - Then, per argument: a 4-byte big-endian length, followed by that many
//!   raw bytes.

/// Protocol version, matching `AMP_VERSION` in `amp.h`.
pub const AMP_VERSION: u8 = 1;

/// Decoded message state, modeling the C `amp_t` struct.
///
/// `version` and `argc` come from the header byte; the remaining payload is
/// held in `buf` with `pos` acting as the cursor that `amp_decode_arg`
/// advances (the C `buf` pointer).
pub struct AmpMessage {
    pub version: u16,
    pub argc: u16,
    buf: Vec<u8>,
    pos: usize,
}

/// Equivalent of C `amp_encode(char **argv, int argc)`.
///
/// Returns the full encoded buffer: header byte, then per argument a
/// 4-byte big-endian length followed by the argument bytes.
pub fn amp_encode(argv: &[&str]) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        1 + argv.iter().map(|s| 4 + s.len()).sum::<usize>(),
    );
    out.push((AMP_VERSION << 4) | (argv.len() as u8 & 0xf));
    for arg in argv {
        let bytes = arg.as_bytes();
        out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        out.extend_from_slice(bytes);
    }
    out
}

/// Equivalent of C `amp_decode(amp_t *msg, char *buf)`.
///
/// Reads the header byte: `version = byte >> 4`, `argc = byte & 0xf`, and
/// positions the cursor just after byte 0.
pub fn amp_decode(buf: &[u8]) -> AmpMessage {
    let first = buf[0];
    let version = (first >> 4) as u16;
    let argc = (first & 0xf) as u16;
    let buf = buf[1..].to_vec();
    AmpMessage {
        version,
        argc,
        buf,
        pos: 0,
    }
}

/// Equivalent of C `amp_decode_arg(amp_t *msg)`.
///
/// Reads the 4-byte big-endian length at the cursor, copies that many bytes
/// into a new `Vec<u8>` (the C `malloc` + `memcpy`), and advances the cursor.
pub fn amp_decode_arg(msg: &mut AmpMessage) -> Vec<u8> {
    let len = u32::from_be_bytes(
        msg.buf[msg.pos..msg.pos + 4]
            .try_into()
            .expect("cursor misaligned: fewer than 4 length bytes left"),
    ) as usize;
    msg.pos += 4;
    let arg = msg.buf[msg.pos..msg.pos + len].to_vec();
    msg.pos += len;
    arg
}
