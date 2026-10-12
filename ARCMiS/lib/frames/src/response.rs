//! Command response frame.

use serde::Deserialize;

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
