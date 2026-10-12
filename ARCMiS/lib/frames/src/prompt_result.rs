//! Terminal state of one prompt command.

use serde::Deserialize;

use crate::prompt_error_detail::PromptErrorDetail;

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
