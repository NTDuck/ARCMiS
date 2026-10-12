//! Transport failure classes.

/// Errors a transport may surface, decoupled from the SDK internals.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// The child closed or died.
    #[error("transport closed")]
    Closed,
    /// A request could not be written.
    #[error("send failed: {0}")]
    Send(String),
    /// A wait timed out.
    #[error("recv timed out")]
    Timeout,
    /// The transport refused the payload shape.
    #[error("bad request: {0}")]
    BadRequest(String),
}
