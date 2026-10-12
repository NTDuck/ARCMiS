//! Forwarded session event of one subagent.

use serde::Deserialize;

use crate::session_event::SessionEvent;

/// One forwarded session event of one subagent. The payload has NO `agent`
/// field. Route by `payload.id` through an id-to-agent map.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentEventFrame {
    /// Event payload.
    pub payload: SubagentEventPayload,
}

/// Body of `subagent_event`. The `event` field is a full session frame.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentEventPayload {
    /// Subagent id.
    pub id: String,
    /// Forwarded session event.
    pub event: SessionEvent,
}
