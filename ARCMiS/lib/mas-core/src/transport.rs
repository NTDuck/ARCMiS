//! `transport` is the seam between the conductor and the omp child (ADR
//! 0034).
//!
//! The conductor drives one run through `send` and `recv` only. The
//! default implementation wraps one `omprpc::Session`. Tests swap in a
//! scripted transport. The trait stays object-safe: one run holds one
//! boxed transport.

use std::future::Future;
use std::time::Duration;

use omprpc::config::PromptRequest;
use omprpc::config::SessionConfig;
use omprpc::frame::Event;
use omprpc::Session;

/// Errors a transport may surface, decoupled from omprpc internals.
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

/// One bidirectional frame stream. `send` is fire-and-forget: terminal
/// frames arrive through `recv`.
pub trait Transport {
    /// Submit one prompt. Returns the request id at once.
    fn send_prompt(&mut self, message: &str) -> impl Future<Output = Result<String, TransportError>> + Send;

    /// Receive the next decoded event.
    fn recv(&mut self) -> impl Future<Output = Result<Event, TransportError>> + Send;

    /// Receive the next event with a deadline. `Timeout` on expiry.
    fn recv_within(&mut self, timeout: Duration) -> impl Future<Output = Result<Event, TransportError>> + Send;
}

/// The default transport over one omprpc session. `Session` holds no
/// `Debug` impl, so the transport reports its shape through `spawn`
/// outcomes instead.
pub struct OmpTransport {
    session: Session,
}

impl OmpTransport {
    /// Spawn the omp child.
    pub fn spawn(config: &SessionConfig) -> Result<Self, TransportError> {
        Session::spawn(config)
            .map(|session| Self {
                session,
            })
            .map_err(|error| match error {
                omprpc::Error::TransportClosed | omprpc::Error::HandshakeFailed(_) => TransportError::Closed,
                other => TransportError::Send(other.to_string()),
            })
    }
}

impl Transport for OmpTransport {
    async fn send_prompt(&mut self, message: &str) -> Result<String, TransportError> {
        self.session.prompt(PromptRequest::new(message)).await.map_err(|error| match error {
            omprpc::Error::TransportClosed => TransportError::Closed,
            other => TransportError::Send(other.to_string()),
        })
    }

    async fn recv(&mut self) -> Result<Event, TransportError> {
        self.session.next_event().await.ok_or(TransportError::Closed)
    }

    async fn recv_within(&mut self, timeout: Duration) -> Result<Event, TransportError> {
        self.session.next_event_within(timeout).await.ok_or(TransportError::Timeout)
    }
}
