//! `omprpc` drives one `omp --mode rpc` child over newline-delimited JSON.
//!
//! The crate offers:
//! - [`SessionConfig`] — knobs for the child process.
//! - [`Session`] — spawn, handshake, fire-and-forget requests, event stream.
//! - [`Event`] — typed server frames plus a tolerant catch-all.
//! - [`Error`] — protocol-level failures a caller must match.
//!
//! Re-exports:
// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod error;
pub mod frame;
pub mod frame_payloads;
pub mod session;

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;

pub use config::{FrameLimits, SessionConfig};
pub use error::Error;
pub use frame::{decode_line, Event, RawLine};
pub use frame_payloads::{
    AgentProgress, ExtensionUiRequestFrame, PromptResultFrame, ReadyFrame, ResponseFrame, SessionEvent,
    StreamingBehavior, SubagentEventFrame, SubagentEventPayload, SubagentLifecycleFrame, SubagentLifecyclePayload,
    SubagentProgressFrame, SubagentProgressPayload,
};
pub use session::Session;
