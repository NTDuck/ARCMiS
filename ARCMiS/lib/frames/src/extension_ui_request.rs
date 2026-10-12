//! Dialog request from an extension.

use serde::Deserialize;

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
