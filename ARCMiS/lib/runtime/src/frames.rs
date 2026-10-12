//! Wire frame vocabulary over the omp RPC child (ADR 0035).
//!
//! The xoannova `omp-rpc` SDK decodes the wire. This module keeps the
//! local frame types the conductor and frame sink consume, plus the
//! mapping from one SDK event onto them.
//!
//! Field names follow the live wire: ALL camelCase. A missing
//! `#[serde(rename)]` does not fail parsing when a catch-all variant
//! absorbs the frame — it silently drops the data. The shapes come
//! from a live census against omp 18.6.1, never from memory.

use serde::Deserialize;

pub use crate::agent_progress::AgentProgress;
pub use crate::extension_ui_request::ExtensionUiRequestFrame;
pub use crate::prompt_error_detail::PromptErrorDetail;
pub use crate::prompt_result::PromptResultFrame;
pub use crate::ready::ReadyFrame;
pub use crate::response::ResponseFrame;
pub use crate::session_event::SessionEvent;
pub use crate::subagent_event::{SubagentEventFrame, SubagentEventPayload};
pub use crate::subagent_lifecycle::{SubagentLifecycleFrame, SubagentLifecyclePayload};
pub use crate::subagent_progress::{SubagentProgressFrame, SubagentProgressPayload};

/// One decoded server frame. Unknown shapes stay typed as [`Event::Other`].
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// First frame on stdout. Carries the protocol version and frame limits.
    Ready(ReadyFrame),
    /// Command response. `id` echoes the request id the host sent.
    Response(ResponseFrame),
    /// Terminal state of one prompt command.
    PromptResult(PromptResultFrame),
    /// Registry transition for one subagent. Route by `payload.agent`.
    SubagentLifecycle(SubagentLifecycleFrame),
    /// Full progress snapshot for one subagent. Route by `payload.agent`.
    SubagentProgress(SubagentProgressFrame),
    /// One forwarded session event of one subagent. The payload has NO `agent`
    /// field. Route by `payload.id` through an id-to-agent map.
    SubagentEvent(SubagentEventFrame),
    /// Dialog request from an extension. Answer with `extension_ui_response`.
    ExtensionUiRequest(ExtensionUiRequestFrame),
    /// Catch-all for frame types this crate does not model.
    #[serde(untagged)]
    Other(SessionEvent),
}
