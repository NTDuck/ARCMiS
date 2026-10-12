//! Advisor failure classes.
//!
//! Why a consultation did not produce an answer.

use crate::transport_error::TransportError;

/// Why a consultation did not produce an answer.
#[derive(Debug, thiserror::Error)]
pub enum AdvisorError {
    /// The advisor is disabled in the run config.
    #[error("advisor disabled")]
    Disabled,
    /// The transport refused the consultation.
    #[error("transport error: {0}")]
    TransportError(#[from] TransportError),
    /// The backlog is full. The caller should defer.
    #[error("advisor backlog full")]
    BacklogFull,
}
