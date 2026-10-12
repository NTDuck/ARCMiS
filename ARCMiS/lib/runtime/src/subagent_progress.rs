//! Subagent progress frame.

use serde::Deserialize;

use crate::agent_progress::AgentProgress;

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
