//! The top-level decoded server frame.

use serde::Deserialize;

use crate::extension_ui_request::ExtensionUiRequestFrame;
use crate::prompt_result::PromptResultFrame;
use crate::ready::ReadyFrame;
use crate::response::ResponseFrame;
use crate::session_event::SessionEvent;
use crate::subagent_event::SubagentEventFrame;
use crate::subagent_lifecycle::SubagentLifecycleFrame;
use crate::subagent_progress::SubagentProgressFrame;

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
