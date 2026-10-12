//! `mas_blackboard` holds per-topic state as bounded revision chains (ADR 0031).
//!
//! Each topic keeps every published revision. `latest` reads the head.
//! The revision bound trims from the front. Memory stays flat, and the
//! trim keeps history order.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod blackboard;
pub mod revision;
pub mod topic;

pub use blackboard::Blackboard;
pub use revision::Revision;
