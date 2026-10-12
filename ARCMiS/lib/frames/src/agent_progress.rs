//! Live counters and current activity of one agent.

use serde::Deserialize;

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
