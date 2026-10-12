//! Configuration for the JEV decision service.

/// Configuration for the JEV decision service.
#[derive(Debug, Clone)]
pub struct JevConfig {
    /// Confidence threshold below which the service falls back to
    /// deterministic rules.
    pub confidence_threshold: f64,
    /// Whether the service is enabled.
    pub enabled: bool,
}

impl Default for JevConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.7,
            enabled: true,
        }
    }
}
