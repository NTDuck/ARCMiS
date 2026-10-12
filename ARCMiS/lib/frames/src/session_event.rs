//! Catch-all payload for frame types this crate does not model.

use serde::Deserialize;

/// Catch-all payload for frame types this crate does not model.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SessionEvent {
    /// Raw frame fields preserved verbatim.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
