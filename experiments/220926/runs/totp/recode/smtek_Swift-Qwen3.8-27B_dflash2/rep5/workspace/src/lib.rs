//! TOTP (time-based one-time passwords), from scratch, dependency-free.
//!
//! Rust translation of the C implementation by Sijmen Mulder (BSD-2-Clause).
//! See `src/totp.rs` for the algorithms.

pub mod totp;

pub use totp::{
    from_base32, hmac_sha1, hotp, pack32, rotl, sha1, totp, unpack32, unpack64,
};

/// C `TOTP_OK` status code (kept for fidelity; the Rust API uses
/// `Option`/`Result` instead of status codes).
pub const TOTP_OK: i32 = 0;

/// C `TOTP_EBOUNDS` status code: argument out-of-bounds / overflow.
pub const TOTP_EBOUNDS: i32 = 1;
