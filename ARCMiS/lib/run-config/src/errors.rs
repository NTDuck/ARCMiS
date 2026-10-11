//! Validation finding type.

use std::fmt;

/// One rejected field. The reason says what to fix.
///
/// `Display` joins the field and the reason. That keeps `anyhow` contexts
/// and test assertions on one readable string.
#[derive(Debug, PartialEq, Eq)]
pub struct ValidationError {
    /// Dotted path of the rejected field, for example `paths.input_dir`.
    pub field: &'static str,
    /// Actionable statement of what to set.
    pub reason: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.reason)
    }
}

impl std::error::Error for ValidationError {}
