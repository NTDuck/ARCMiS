//! `mas_frames` is the wire vocabulary over the omp RPC child (ADR 0035).
//!
//! The xoannova `omp-rpc` SDK decodes the wire. This crate keeps the
//! local frame types the conductor and frame sink consume, plus the
//! mapping from one SDK event onto them.
//!
//! Field names follow the live wire: ALL camelCase. A missing
//! `#[serde(rename)]` does not fail parsing when a catch-all variant
//! absorbs the frame — it silently drops the data. The shapes come
//! from a live census against omp 18.6.1, never from memory.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod agent_progress;
pub mod event;
pub mod extension_ui_request;
pub mod prompt_error_detail;
pub mod prompt_result;
pub mod ready;
pub mod response;
pub mod session_event;
pub mod subagent_event;
pub mod subagent_lifecycle;
pub mod subagent_progress;

mod map;

pub use agent_progress::AgentProgress;
pub use event::Event;
pub use extension_ui_request::ExtensionUiRequestFrame;
pub use map::map_sdk_event;
pub use prompt_error_detail::PromptErrorDetail;
pub use prompt_result::PromptResultFrame;
pub use ready::ReadyFrame;
pub use response::ResponseFrame;
pub use session_event::SessionEvent;
pub use subagent_event::{SubagentEventFrame, SubagentEventPayload};
pub use subagent_lifecycle::{SubagentLifecycleFrame, SubagentLifecyclePayload};
pub use subagent_progress::{SubagentProgressFrame, SubagentProgressPayload};
