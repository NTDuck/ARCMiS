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

use serde::Deserialize;
use serde_json::Value;

use omp_rpc::Event as SdkEvent;
use omp_rpc::RpcNotification;

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

/// Map one SDK event onto the local vocabulary.
pub fn map_sdk_event(event: SdkEvent) -> Event {
    match event {
        SdkEvent::Notification(notification) => map_notification(notification),
        SdkEvent::UnmatchedResponse(response) => Event::Response(ResponseFrame {
            id: response.id,
            command: Some(response.command),
            success: response.success,
            data: response.data,
            error: response.error.map(Value::String),
        }),
        SdkEvent::Undecodable {
            raw,
            error,
        } => {
            // WHY a catch-all: omp adds frame types between releases and
            // the SDK refuses the unknown shape. Dropping them silently
            // hangs wait loops (verified in a prior integration), so
            // unknown frames stay reachable as the raw payload and log
            // at debug.
            tracing::debug!(error = %error, "kept undecodable frame");
            Event::Other(raw_event(raw))
        },
    }
}

fn map_notification(notification: RpcNotification) -> Event {
    match notification {
        RpcNotification::Ready(ready) => Event::Ready(ReadyFrame {
            protocol_version: ready.protocol_version.unwrap_or(1) as u32,
            supported_protocol_versions: ready
                .supported_protocol_versions
                .map(|versions| versions.iter().map(|version| *version as u32).collect())
                .unwrap_or_default(),
            max_frame_bytes: ready.max_frame_bytes.unwrap_or(0) as u64,
            max_reassembled_frame_bytes: ready.max_reassembled_frame_bytes.unwrap_or(0) as u64,
        }),
        RpcNotification::PromptResult(result) => Event::PromptResult(PromptResultFrame {
            id: result.id,
            agent_invoked: Some(result.agent_invoked),
            status: Some(result.status.as_str().to_owned()),
            session_settled: Some(result.session_settled),
            error: result.error.map(|error| PromptErrorDetail {
                message: error.message,
                retryable: error.retryable,
                provider: error.provider,
                model: error.model,
                http_status: error.http_status,
            }),
        }),
        RpcNotification::SubagentLifecycle(frame) => Event::SubagentLifecycle(SubagentLifecycleFrame {
            payload: SubagentLifecyclePayload {
                id: frame.payload.id,
                agent: frame.payload.agent,
                agent_source: Some(frame.payload.agent_source.as_str().to_owned()),
                description: frame.payload.description,
                status: frame.payload.status.as_str().to_owned(),
                session_file: frame.payload.session_file,
                parent_tool_call_id: frame.payload.parent_tool_call_id,
                index: frame.payload.index as u32,
                detached: frame.payload.detached,
            },
        }),
        RpcNotification::SubagentProgress(frame) => Event::SubagentProgress(SubagentProgressFrame {
            payload: SubagentProgressPayload {
                index: frame.payload.index as u32,
                agent: frame.payload.agent,
                agent_source: Some(frame.payload.agent_source.as_str().to_owned()),
                task: Some(frame.payload.task),
                assignment: frame.payload.assignment,
                progress: serde_json::from_value(Value::Object(frame.payload.progress.clone())).unwrap_or_default(),
                session_file: frame.payload.session_file,
                detached: frame.payload.detached,
            },
        }),
        RpcNotification::SubagentEvent(frame) => Event::SubagentEvent(SubagentEventFrame {
            payload: SubagentEventPayload {
                id: frame.payload.id,
                event: raw_event(serde_json::to_value(&frame.payload.event).unwrap_or(Value::Null)),
            },
        }),
        RpcNotification::ExtensionUiRequest(request) => Event::ExtensionUiRequest(
            serde_json::from_value(serde_json::to_value(&request).unwrap_or(Value::Null)).unwrap_or_default(),
        ),
        RpcNotification::RpcAgentEvent(agent_event) => {
            Event::Other(raw_event(serde_json::to_value(&agent_event).unwrap_or(Value::Null)))
        },
        RpcNotification::Unknown(raw) => Event::Other(raw_event(raw)),
        other => Event::Other(raw_event(serde_json::to_value(&other).unwrap_or(Value::Null))),
    }
}

/// Fold one raw frame value into the catch-all payload.
fn raw_event(value: Value) -> SessionEvent {
    let extra = match value {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    SessionEvent {
        extra,
    }
}

/// Catch-all payload for frame types this crate does not model.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SessionEvent {
    /// Raw frame fields preserved verbatim.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// First frame on stdout. Carries the protocol version and frame limits.
#[derive(Debug, Clone, Deserialize)]
pub struct ReadyFrame {
    /// Protocol version the server speaks.
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
    /// Protocol versions the server accepts.
    #[serde(rename = "supportedProtocolVersions")]
    pub supported_protocol_versions: Vec<u32>,
    /// Maximum single-frame byte size.
    #[serde(rename = "maxFrameBytes")]
    pub max_frame_bytes: u64,
    /// Maximum reassembled-frame byte size.
    #[serde(rename = "maxReassembledFrameBytes")]
    pub max_reassembled_frame_bytes: u64,
}

/// Command response. `id` echoes the request id the host sent.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseFrame {
    /// Request id echoed from the host.
    #[serde(default)]
    pub id: Option<String>,
    /// Command name that produced the response.
    #[serde(default)]
    pub command: Option<String>,
    /// Whether the command succeeded.
    #[serde(default)]
    pub success: bool,
    /// Optional response payload.
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    /// Optional error payload.
    #[serde(default)]
    pub error: Option<serde_json::Value>,
}

/// Failure detail of one prompt result with a failing status. Mirrors
/// the wire `error` object of `prompt_result`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct PromptErrorDetail {
    /// Human-readable error message.
    #[serde(default)]
    pub message: String,
    /// Whether the caller may retry.
    #[serde(default)]
    pub retryable: bool,
    /// Provider that surfaced the error.
    #[serde(default)]
    pub provider: Option<String>,
    /// Model that surfaced the error.
    #[serde(default)]
    pub model: Option<String>,
    /// HTTP status code when the error came from a remote call.
    #[serde(rename = "httpStatus", default)]
    pub http_status: Option<i64>,
}

/// Terminal state of one prompt command.
#[derive(Debug, Clone, Deserialize)]
pub struct PromptResultFrame {
    /// Request id echoed from the host.
    #[serde(default)]
    pub id: Option<String>,
    /// Whether the agent was invoked.
    #[serde(rename = "agentInvoked", default)]
    pub agent_invoked: Option<bool>,
    /// Wire status string.
    #[serde(default)]
    pub status: Option<String>,
    /// Whether the session settled after the prompt.
    #[serde(rename = "sessionSettled", default)]
    pub session_settled: Option<bool>,
    /// Failure detail when the prompt failed.
    #[serde(default)]
    pub error: Option<PromptErrorDetail>,
}

/// Registry transition for one subagent. Route by `payload.agent`.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentLifecycleFrame {
    /// Lifecycle payload.
    pub payload: SubagentLifecyclePayload,
}

/// Body of `subagent_lifecycle`.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentLifecyclePayload {
    /// Subagent id.
    pub id: String,
    /// Subagent role name.
    pub agent: String,
    /// Source of the agent definition.
    #[serde(rename = "agentSource", default)]
    pub agent_source: Option<String>,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Lifecycle status string.
    pub status: String,
    /// Session file path.
    #[serde(rename = "sessionFile", default)]
    pub session_file: Option<String>,
    /// Parent tool call id.
    #[serde(rename = "parentToolCallId", default)]
    pub parent_tool_call_id: Option<String>,
    /// Subagent index.
    #[serde(default)]
    pub index: u32,
    /// Whether the subagent is detached.
    #[serde(default)]
    pub detached: Option<bool>,
}

/// Full progress snapshot for one subagent. Route by `payload.agent`.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentProgressFrame {
    /// Progress payload.
    #[serde(flatten)]
    pub payload: SubagentProgressPayload,
}

/// Body of `subagent_progress`. The `progress` field mirrors the Agent Hub row.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentProgressPayload {
    /// Subagent index.
    #[serde(default)]
    pub index: u32,
    /// Subagent role name.
    pub agent: String,
    /// Source of the agent definition.
    #[serde(rename = "agentSource", default)]
    pub agent_source: Option<String>,
    /// Current task description.
    #[serde(default)]
    pub task: Option<String>,
    /// Current assignment description.
    #[serde(default)]
    pub assignment: Option<String>,
    /// Live counters and current activity.
    pub progress: AgentProgress,
    /// Session file path.
    #[serde(rename = "sessionFile", default)]
    pub session_file: Option<String>,
    /// Whether the subagent is detached.
    #[serde(default)]
    pub detached: Option<bool>,
}

/// Live counters and current activity of one agent.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgentProgress {
    /// Subagent id.
    #[serde(default)]
    pub id: Option<String>,
    /// Current status string.
    #[serde(default)]
    pub status: Option<String>,
    /// Last intent the agent emitted.
    #[serde(rename = "lastIntent", default)]
    pub last_intent: Option<String>,
    /// Tool the agent is currently running.
    #[serde(rename = "currentTool", default)]
    pub current_tool: Option<String>,
    /// Arguments for the current tool.
    #[serde(rename = "currentToolArgs", default)]
    pub current_tool_args: Option<String>,
    /// Recent output lines.
    #[serde(rename = "recentOutput", default)]
    pub recent_output: Vec<String>,
    /// Tool call count.
    #[serde(rename = "toolCount", default)]
    pub tool_count: u32,
    /// Tokens consumed.
    #[serde(default)]
    pub tokens: u64,
    /// Context tokens consumed.
    #[serde(rename = "contextTokens", default)]
    pub context_tokens: Option<u64>,
    /// Context window size.
    #[serde(rename = "contextWindow", default)]
    pub context_window: Option<u64>,
    /// Cost in dollars.
    #[serde(default)]
    pub cost: f64,
    /// Duration in milliseconds.
    #[serde(rename = "durationMs", default)]
    pub duration_ms: u64,
}

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

/// Dialog request from an extension. Answer with `extension_ui_response`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExtensionUiRequestFrame {
    /// Request id.
    #[serde(default)]
    pub id: Option<String>,
    /// Method name.
    #[serde(default)]
    pub method: Option<String>,
    /// Extra fields preserved verbatim.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
