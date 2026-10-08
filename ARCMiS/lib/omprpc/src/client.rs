//! `client` pairs request ids with responses on top of [`Transport`].

use crate::frame::ServerFrame;
use crate::transport::{SpawnConfig, Transport};
use anyhow::Context;
use std::time::Duration;
use tokio::sync::mpsc;

/// Default wait for plain command responses. Prompt waits are separate.
pub const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

/// Thin typed wrapper over the RPC command set this TUI drives.
///
/// The TUI owns the event stream. It forwards every frame it sees to
/// [`Client::feed_frame`]. Command waits then correlate against a mirror
/// of the live stream instead of holding the transport receiver.
pub struct Client {
    pub transport: Transport,
    frame_tx: mpsc::UnboundedSender<ServerFrame>,
    frame_rx: mpsc::UnboundedReceiver<ServerFrame>,
    next_id: u64,
}

impl Client {
    /// Spawn the child and negotiate protocol v2.
    pub async fn connect(config: SpawnConfig) -> anyhow::Result<Self> {
        let mut transport = Transport::spawn(config)?;
        let ready = transport.ready().await?;
        let (frame_tx, frame_rx) = mpsc::unbounded_channel();
        let mut client = Self {
            transport,
            frame_tx,
            frame_rx,
            next_id: 1,
        };
        let request_id = client
            .request(serde_json::json!({
                "type": "negotiate_protocol",
                "protocolVersion": 2,
            }))
            .await
            .context("protocol negotiation")?;
        let deadline = tokio::time::Instant::now() + RESPONSE_TIMEOUT;
        let negotiated = loop {
            let frame = client.transport.read_event_before(deadline).await?;
            if let ServerFrame::Response(response) = frame {
                if response.id.as_deref() == Some(request_id.as_str()) {
                    break response;
                }
            }
        };
        anyhow::ensure!(negotiated.success, "protocol negotiation failed");
        tracing::debug!(advertised = ready.max_frame_bytes, "negotiated protocol v2");
        Ok(client)
    }

    /// Mirror one frame into the correlation buffer. Call for every frame.
    pub fn feed_frame(&mut self, frame: ServerFrame) {
        let _ = self.frame_tx.send(frame);
    }

    /// Forward one request line with a fresh id. Fire and forget.
    pub async fn request(&mut self, payload: serde_json::Value) -> anyhow::Result<String> {
        let id = format!("req-{}", self.next_id);
        self.next_id += 1;
        let mut value = payload;
        if let Some(object) = value.as_object_mut() {
            object.insert("id".to_string(), serde_json::json!(id));
        }
        self.transport.requests.send(serde_json::to_string(&value)?).await.context("request channel closed")?;
        Ok(id)
    }

    /// Send one command and wait for the matching `response` frame.
    pub async fn command(
        &mut self,
        command_type: &str,
        params: Option<serde_json::Value>,
        timeout: Duration,
    ) -> anyhow::Result<crate::frame::ResponseFrame> {
        let mut payload = match params {
            Some(params) => params,
            None => serde_json::json!({}),
        };
        if let Some(object) = payload.as_object_mut() {
            object.insert("type".to_string(), serde_json::json!(command_type));
        } else {
            payload = serde_json::json!({ "type": command_type });
        }
        let id = self.request(payload).await?;
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let frame = self.next_frame_before(deadline).await?;
            if let ServerFrame::Response(response) = frame {
                if response.id.as_deref() == Some(id.as_str()) {
                    return Ok(response);
                }
            }
        }
    }

    /// Submit a prompt. Returns once the terminal `prompt_result` arrives.
    pub async fn prompt(&mut self, message: &str, streaming_behavior: Option<&str>) -> anyhow::Result<()> {
        let mut params = serde_json::json!({ "type": "prompt", "message": message });
        if let Some(streaming_behavior) = streaming_behavior {
            params["streamingBehavior"] = serde_json::json!(streaming_behavior);
        }
        let id = self.request(params).await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3600);
        loop {
            let frame = self.next_frame_before(deadline).await?;
            if let ServerFrame::PromptResult(prompt_result) = frame {
                if prompt_result.id.as_deref() == Some(id.as_str()) {
                    return Ok(());
                }
            }
        }
    }

    /// Abort the current run, then submit a new prompt.
    pub async fn abort_and_prompt(&mut self, message: &str) -> anyhow::Result<()> {
        let params = serde_json::json!({ "type": "abort_and_prompt", "message": message });
        let id = self.request(params).await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3600);
        loop {
            let frame = self.next_frame_before(deadline).await?;
            if let ServerFrame::PromptResult(prompt_result) = frame {
                if prompt_result.id.as_deref() == Some(id.as_str()) {
                    return Ok(());
                }
            }
        }
    }

    async fn next_frame_before(&mut self, deadline: tokio::time::Instant) -> anyhow::Result<ServerFrame> {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        tokio::time::timeout(remaining, self.frame_rx.recv())
            .await
            .context("timed out waiting for the next frame")?
            .context("frame mirror closed")
    }
}
