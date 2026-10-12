//! The default transport over one SDK session.
//!
//! `Client` holds no `Debug` impl, so the transport reports its shape
//! through `spawn` outcomes instead.

use std::collections::VecDeque;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use omp_rpc::Client;
use omp_rpc::ClientOptions;
use omp_rpc::Error as SdkError;
use omp_rpc::PromptCommand;
use omp_rpc::SetSubagentSubscriptionCommand;

use crate::forward_frames::forward_frames;
use crate::frames::Event;
use crate::omp_launch_config::OmpLaunchConfig;
use crate::sdk_error_map::map_sdk_error;
use crate::transport_error::TransportError;
use crate::transport_trait::Transport;

/// The default transport over one SDK session. `Client` holds no
/// `Debug` impl, so the transport reports its shape through `spawn`
/// outcomes instead.
pub struct OmpTransport {
    client: Arc<Client>,
    events: tokio::sync::mpsc::Receiver<Event>,
    /// Prompt tokens waiting for their terminal `prompt_result`, oldest
    /// first. The session processes prompts serially, so completion
    /// order equals submission order.
    pending_prompts: VecDeque<String>,
    /// Next numeric token, mirroring the retired in-tree client ids.
    next_token: u64,
}

impl OmpTransport {
    /// Spawn the omp child.
    pub fn spawn(config: &OmpLaunchConfig) -> Result<Self, TransportError> {
        let mut command = Command::new(&config.program);
        command
            .args(config.argv())
            .current_dir(&config.working_dir)
            .envs(config.env.iter().map(|(key, value)| (key.as_str(), value.as_str())))
            .stderr(Stdio::null());
        // WHY ClientOptions::default(): the SDK ready (60s) and command
        // (30s) deadlines are upstream protocol defaults, not
        // caller-tuned values. Adding timeout fields here would add
        // surface nobody sets.
        let (client, frames) = Client::spawn(command, ClientOptions::default()).map_err(|error| match error {
            // WHY Closed: a binary that cannot launch never opened the
            // transport, matching the retired client's handshake error.
            SdkError::Io(_) => TransportError::Closed,
            other => map_sdk_error(other),
        })?;
        let client = Arc::new(client);
        // WHY a subscription call: the server forwards subagent frames
        // only after set_subagent_subscription. Without it the subagent
        // stream stays silent and nothing errors (live-wire verified).
        client
            .call(&SetSubagentSubscriptionCommand {
                level: config.subagent_subscription,
            })
            .map_err(map_sdk_error)?;
        let (sender, receiver) = tokio::sync::mpsc::channel(256);
        std::thread::spawn(move || forward_frames(frames, &sender));
        Ok(Self {
            client,
            events: receiver,
            pending_prompts: VecDeque::new(),
            next_token: 1,
        })
    }

    /// Re-key the terminal prompt frame to the token `send_prompt`
    /// returned. The SDK assigns its own internal ids (`req_N`) and
    /// never hands them out, so the transport issues numeric tokens of
    /// its own and matches them on delivery. The session processes
    /// prompts serially, so completion order equals submission order and
    /// the oldest pending token is the right one.
    fn rekey_prompt(&mut self, frame: Event) -> Event {
        let Event::PromptResult(mut result) = frame else {
            return frame;
        };
        if let Some(token) = self.pending_prompts.pop_front() {
            result.id = Some(token);
        }
        Event::PromptResult(result)
    }
}

impl Transport for OmpTransport {
    async fn send_prompt(&mut self, message: &str) -> Result<String, TransportError> {
        // WHY the token registers before the call: the terminal frame
        // follows the admission ack on the wire, so the token is in place
        // before any `prompt_result` can reach `recv` (ADR 0035).
        let token = self.next_token.to_string();
        self.next_token += 1;
        self.pending_prompts.push_back(token.clone());
        let prompt = PromptCommand {
            message: message.to_owned(),
            images: None,
            streaming_behavior: None,
        };
        let client = Arc::clone(&self.client);
        // WHY spawn_blocking: the SDK call blocks its thread on the
        // admission ack. Running it on a runtime worker would starve the
        // event loop the frame stream depends on (the ADR 0034 deadlock).
        let ack = tokio::task::spawn_blocking(move || client.call(&prompt))
            .await
            .map_err(|error| TransportError::Send(error.to_string()))?
            .map_err(map_sdk_error)?;
        // WHY the pop: a locally handled prompt sends no terminal frame
        // (agentInvoked false), so its token must not claim a later one.
        if ack.agent_invoked == Some(false) {
            self.pending_prompts.pop_front();
        }
        Ok(token)
    }

    async fn recv(&mut self) -> Result<Event, TransportError> {
        let frame = self.events.recv().await.ok_or(TransportError::Closed)?;
        Ok(self.rekey_prompt(frame))
    }

    async fn recv_within(&mut self, timeout: Duration) -> Result<Event, TransportError> {
        match tokio::time::timeout(timeout, self.events.recv()).await {
            Ok(Some(frame)) => Ok(self.rekey_prompt(frame)),
            Ok(None) => Err(TransportError::Closed),
            Err(_) => Err(TransportError::Timeout),
        }
    }
}
