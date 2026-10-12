//! Subagent lifecycle frame.

use serde::Deserialize;

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
