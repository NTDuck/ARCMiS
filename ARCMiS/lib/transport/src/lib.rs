//! `mas_transport` is the seam between the conductor and the omp child
//! (ADR 0034, ADR 0035).
//!
//! The conductor drives one run through `send` and `recv` only. The
//! default implementation wraps one `omp_rpc::Client` session from the
//! xoannova oh-my-pi-omp SDK (ADR 0035). Tests swap in a scripted
//! transport. The trait stays object-safe: one run holds one boxed
//! transport.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

use std::collections::VecDeque;
use std::future::Future;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use omp_rpc::Client;
use omp_rpc::ClientOptions;
use omp_rpc::Error as SdkError;
use omp_rpc::Event as SdkEvent;
use omp_rpc::PromptCommand;
use omp_rpc::SetSubagentSubscriptionCommand;
use omp_rpc::SubagentSubscriptionLevel;

use frames::Event;
use frames::map_sdk_event;

pub use error::TransportError;

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

/// Knobs for the child process. The caller owns every value.
#[derive(Debug, Clone)]
pub struct OmpLaunchConfig {
    /// Binary to launch. The host wrapper (`ompl`) sits here, so the
    /// library never names a binary itself (config rule).
    pub program: String,
    /// Session model, passed as `--model=<value>`.
    pub model: Option<String>,
    /// Tool approval mode, passed as `--approval-mode=<value>`.
    pub approval_mode: String,
    /// Extra arguments appended after the fixed flags.
    pub extra_args: Vec<String>,
    /// Working directory of the child.
    pub working_dir: PathBuf,
    /// Environment variables for the child.
    pub env: Vec<(String, String)>,
    /// Subagent frames the server forwards after the handshake.
    pub subagent_subscription: SubagentSubscriptionLevel,
    /// Disable every built-in tool. The advisor uses this to keep the
    /// session read-only: no file writes, no command execution.
    pub read_only: bool,
}

impl Default for OmpLaunchConfig {
    fn default() -> Self {
        Self {
            program: "omp".to_string(),
            model: None,
            approval_mode: "yolo".to_string(),
            extra_args: Vec::new(),
            // WHY a literal: the config rule bans environment reads in
            // library code. `Command::current_dir(".")` inherits the
            // parent cwd at spawn time, so the behavior matches a plain
            // shell launch.
            working_dir: PathBuf::from("."),
            env: Vec::new(),
            // WHY events: the frame vocabulary routes subagent panes by
            // `payload.agent` and `payload.id`. Only the events level
            // forwards the raw session events the router needs (ADR 0030).
            subagent_subscription: SubagentSubscriptionLevel::Events,
            read_only: false,
        }
    }
}

impl OmpLaunchConfig {
    /// Build the child argv: mode, model, approval mode, passthrough args.
    ///
    /// WHY `--model`: omp selects the session model with
    /// `--model=<value>` (fuzzy match, `provider/model`). The form comes
    /// from `omp --help`. No RPC-level model message exists.
    ///
    /// WHY `--no-tools`: a read-only session disables every built-in
    /// tool at the CLI level. The advisor cannot write files or run
    /// commands even if a prompt asks for it (ADR 0036 section 6).
    pub fn argv(&self) -> Vec<String> {
        let mut argv = vec!["--mode".to_string(), "rpc".to_string()];
        if let Some(model) = &self.model {
            argv.push(format!("--model={model}"));
        }
        argv.push(format!("--approval-mode={}", self.approval_mode));
        if self.read_only {
            argv.push("--no-tools".to_string());
        }
        argv.extend(self.extra_args.iter().cloned());
        argv
    }
}

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
}

/// Map one SDK error onto the local transport error.
pub fn map_sdk_error(error: SdkError) -> TransportError {
    match error {
        SdkError::Closed => TransportError::Closed,
        SdkError::Timeout {
            ..
        } => TransportError::Timeout,
        SdkError::Command {
            command,
            error: detail,
            code: Some(code),
        } => TransportError::BadRequest(format!("{command} failed ({code}): {detail}")),
        SdkError::Command {
            command,
            error: detail,
            code: None,
        } => TransportError::BadRequest(format!("{command} failed: {detail}")),
        SdkError::Protocol(detail) => TransportError::BadRequest(detail),
        SdkError::InvalidArgument(detail) => TransportError::BadRequest(detail),
        SdkError::Io(error) => TransportError::Send(error.to_string()),
        SdkError::Json(error) => TransportError::Send(error.to_string()),
    }
}

/// Forward decoded SDK frames into the async stream until the child ends.
fn forward_frames(receiver: std::sync::mpsc::Receiver<SdkEvent>, sender: &tokio::sync::mpsc::Sender<Event>) {
    for frame in receiver.iter() {
        if sender.blocking_send(map_sdk_event(frame)).is_err() {
            return;
        }
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

impl OmpTransport {
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
