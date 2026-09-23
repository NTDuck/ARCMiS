//! AMP — Abstract Message Protocol.
//!
//! A tiny binary protocol: a 1-byte header (`version << 4 | argc`)
//! followed by `argc` frames of `<u32be length><data>`.

pub mod amp;

pub use amp::*;
