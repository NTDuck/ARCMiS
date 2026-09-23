//! From-scratch, dependency-free TOTP implementation (port of the C project).
//!
//! Re-exports the public API from the `totp` module.

mod totp;

pub use totp::*;
