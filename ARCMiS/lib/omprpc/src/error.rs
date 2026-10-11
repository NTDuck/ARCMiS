//! `error` defines the protocol failures a caller must match.

/// Protocol-level error. Callers match on these variants. Everything else
/// propagates as `anyhow` context at the boundary.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The child died or closed stdout before the `ready` frame. The source
    /// carries the spawn or pipe error when the failure happened at spawn.
    #[error("transport closed before the handshake finished")]
    HandshakeFailed(#[source] Option<std::io::Error>),
    /// stdout closed or the child exited mid-session.
    #[error("transport closed")]
    TransportClosed,
    /// A frame exceeded the advertised limits.
    #[error("frame too large: {0} bytes")]
    FrameTooLarge(u64),
    /// The child sent a `response` frame with `success: false`.
    #[error("request {request_id} failed: {detail}")]
    RequestRejected {
        request_id: String,
        detail: String,
    },
}
