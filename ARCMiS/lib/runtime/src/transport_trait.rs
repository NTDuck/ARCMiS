//! One bidirectional frame stream.
//!
//! `send` is fire-and-forget: terminal frames arrive through `recv`.

use std::future::Future;
use std::time::Duration;

use crate::frames::Event;
use crate::transport_error::TransportError;

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
