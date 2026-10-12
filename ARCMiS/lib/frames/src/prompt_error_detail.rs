//! Failure detail of one prompt result.

use serde::Deserialize;

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
