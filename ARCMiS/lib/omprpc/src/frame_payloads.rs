//! `frame_payloads` holds the serde wire structs for each frame type.
//!
//! Field names follow the live wire: ALL camelCase. A missing
//! `#[serde(rename)]` does not fail parsing when a catch-all variant
//! absorbs the frame — it silently drops the data. Fixtures come from a
//! live census against omp 18.6.1, never from memory.

use serde::Deserialize;

/// Catch-all payload for frame types this crate does not model.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SessionEvent {
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// First frame on stdout. Carries the protocol version and frame limits.
#[derive(Debug, Clone, Deserialize)]
pub struct ReadyFrame {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
    #[serde(rename = "supportedProtocolVersions")]
    pub supported_protocol_versions: Vec<u32>,
    #[serde(rename = "maxFrameBytes")]
    pub max_frame_bytes: u64,
    #[serde(rename = "maxReassembledFrameBytes")]
    pub max_reassembled_frame_bytes: u64,
}

/// Command response. `id` echoes the request id the host sent.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseFrame {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
}

/// Terminal state of one prompt command.
#[derive(Debug, Clone, Deserialize)]
pub struct PromptResultFrame {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "agentInvoked", default)]
    pub agent_invoked: Option<bool>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(rename = "sessionSettled", default)]
    pub session_settled: Option<bool>,
}

/// How a prompt joins a running turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StreamingBehavior {
    Steer,
    FollowUp,
}

/// Registry transition for one subagent. Route by `payload.agent`.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentLifecycleFrame {
    pub payload: SubagentLifecyclePayload,
}

/// Body of `subagent_lifecycle`.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentLifecyclePayload {
    pub id: String,
    pub agent: String,
    #[serde(rename = "agentSource", default)]
    pub agent_source: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub status: String,
    #[serde(rename = "sessionFile", default)]
    pub session_file: Option<String>,
    #[serde(rename = "parentToolCallId", default)]
    pub parent_tool_call_id: Option<String>,
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub detached: Option<bool>,
}

/// Full progress snapshot for one subagent. Route by `payload.agent`.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentProgressFrame {
    #[serde(flatten)]
    pub payload: SubagentProgressPayload,
}

/// Body of `subagent_progress`. The `progress` field mirrors the Agent Hub row.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentProgressPayload {
    #[serde(default)]
    pub index: u32,
    pub agent: String,
    #[serde(rename = "agentSource", default)]
    pub agent_source: Option<String>,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub assignment: Option<String>,
    pub progress: AgentProgress,
    #[serde(rename = "sessionFile", default)]
    pub session_file: Option<String>,
    #[serde(default)]
    pub detached: Option<bool>,
}

/// Live counters and current activity of one agent.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentProgress {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(rename = "lastIntent", default)]
    pub last_intent: Option<String>,
    #[serde(rename = "currentTool", default)]
    pub current_tool: Option<String>,
    #[serde(rename = "currentToolArgs", default)]
    pub current_tool_args: Option<String>,
    #[serde(rename = "recentOutput", default)]
    pub recent_output: Vec<String>,
    #[serde(rename = "toolCount", default)]
    pub tool_count: u32,
    #[serde(default)]
    pub tokens: u64,
    #[serde(rename = "contextTokens", default)]
    pub context_tokens: Option<u64>,
    #[serde(rename = "contextWindow", default)]
    pub context_window: Option<u64>,
    #[serde(default)]
    pub cost: f64,
    #[serde(rename = "durationMs", default)]
    pub duration_ms: u64,
}

/// One forwarded session event of one subagent. The payload has NO `agent`
/// field. Route by `payload.id` through an id-to-agent map.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentEventFrame {
    pub payload: SubagentEventPayload,
}

/// Body of `subagent_event`. The `event` field is a full session frame.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentEventPayload {
    pub id: String,
    pub event: SessionEvent,
}

/// Dialog request from an extension. Answer with `extension_ui_response`.
#[derive(Debug, Clone, Deserialize)]
pub struct ExtensionUiRequestFrame {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
