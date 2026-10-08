//! `frame_payloads` holds the serde wire structs for each frame type.

use serde::Deserialize;

/// Catch-all for frame types this host renders generically.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionEvent {
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// One tool call start in the main session.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolExecutionStartFrame {
    #[serde(rename = "toolCallId")]
    pub tool_call_id: String,
    #[serde(rename = "toolName")]
    pub tool_name: String,
    #[serde(default, alias = "args")]
    pub arguments: Option<serde_json::Value>,
    #[serde(default)]
    pub intent: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Registry transition for one subagent.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentLifecycleFrame {
    pub payload: SubagentLifecyclePayload,
}

/// Full progress snapshot for one subagent.
#[derive(Debug, Clone, Deserialize)]
pub struct ReadyFrame {
    pub protocol_version: u32,
    #[serde(rename = "supportedProtocolVersions")]
    pub supported_protocol_versions: Vec<u32>,
    pub max_frame_bytes: u64,
    pub max_reassembled_frame_bytes: u64,
}

/// One piece of a v2 oversized frame.
#[derive(Debug, Clone, Deserialize)]
pub struct RpcChunkFrame {
    #[serde(rename = "chunkId")]
    pub chunk_id: String,
    pub index: u32,
    pub count: u32,
    #[serde(rename = "byteLength")]
    pub byte_length: u64,
    pub data: String,
}

/// Command response. `id` echoes the request id that the host sent.
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
    #[serde(default)]
    pub agent_invoked: Option<bool>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub session_settled: Option<bool>,
}

/// Registry transition for one subagent.
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

/// Full progress snapshot for one subagent.
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
    #[serde(default)]
    pub last_intent: Option<String>,
    #[serde(default)]
    pub current_tool: Option<String>,
    #[serde(default)]
    pub current_tool_args: Option<String>,
    #[serde(default, rename = "recentOutput")]
    pub recent_output: Vec<String>,
    #[serde(default, rename = "toolCount")]
    pub tool_count: u32,
    #[serde(default)]
    pub tokens: u64,
    #[serde(default, rename = "contextTokens")]
    pub context_tokens: Option<u64>,
    #[serde(default, rename = "contextWindow")]
    pub context_window: Option<u64>,
    #[serde(default)]
    pub cost: f64,
    #[serde(default, rename = "durationMs")]
    pub duration_ms: u64,
}

/// One forwarded session event of one subagent.
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
