//! From-scratch, dependency-free implementation of TOTP, including SHA1,
//! for education. TOTP is the most common standard for generating codes in
//! 'Authenticator' apps.
//!
//! See LICENSE.md.

pub mod totp;

pub use totp::{
    from_base32, hotp, hmac_sha1, pack32, rotl, sha1, totp, unpack32, unpack64,
    TOTP_EBOUNDS, TOTP_OK,
};
