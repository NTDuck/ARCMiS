//! From-scratch, dependency-free implementation of
//! [TOTP](https://en.wikipedia.org/wiki/Time-based_one-time_password),
//! including SHA1, for education.

pub mod totp;

pub use totp::*;
