//! `session` spawns one `omp --mode rpc` child and moves frames over channels.

use crate::config::{PromptRequest, SessionConfig};
use crate::error::Error;
use crate::frame::{decode_line, Event, RawLine};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// One live RPC child: fire-and-forget request writer and an event stream.
// The child handle is never read: kill_on_drop(true) reaps the process when
// the Session drops, and the caller can still read its exit status through
// the reader task ending. Keep the handle so the Session owns the process.
pub struct Session {
    requests: mpsc::Sender<String>,
    events: mpsc::UnboundedReceiver<Event>,
    #[allow(dead_code)]
    child: Child,
    reader_task: JoinHandle<()>,
    next_id: u64,
}

impl Session {
    /// Spawn the child. The reader task skips banner lines, decodes frames,
    /// and forwards events into an unbounded channel.
    pub fn spawn(config: &SessionConfig) -> Result<Self, Error> {
        let mut child = tokio::process::Command::new(&config.program)
            .args(config.argv())
            .current_dir(&config.working_dir)
            .envs(config.env.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| Error::HandshakeFailed(Some(error)))?;
        let stdin = child.stdin.take().ok_or(Error::HandshakeFailed(None))?;
        let stdout = child.stdout.take().ok_or(Error::HandshakeFailed(None))?;
        let (request_sender, request_receiver) = mpsc::channel::<String>(64);
        let (event_sender, event_receiver) = mpsc::unbounded_channel::<Event>();
        // The writer task holds the stdin side. Dropping its handle is safe.
        let writer_task = tokio::spawn(write_loop(request_receiver, stdin));
        std::mem::drop(writer_task);
        let reader_task = tokio::spawn(read_loop(stdout, event_sender));
        Ok(Self {
            requests: request_sender,
            events: event_receiver,
            child,
            reader_task,
            next_id: 1,
        })
    }

    /// Receive the next decoded event from the child.
    pub async fn next_event(&mut self) -> Option<Event> {
        self.events.recv().await
    }

    /// Receive one event with a deadline. `None` means timeout.
    pub async fn next_event_within(&mut self, timeout: std::time::Duration) -> Option<Event> {
        tokio::time::timeout(timeout, self.events.recv()).await.ok().flatten()
    }

    /// Send one request line with a fresh id. Fire and forget: the `response`
    /// or `prompt_result` frame arrives on the event stream later.
    ///
    /// WHY fire-and-forget: awaiting `prompt_result` inside a tokio::select!
    /// arm deadlocks — the arm borrows the client while the completion frame
    /// sits in the event stream nobody drains. Callers send here and then
    /// await events in their own select loop.
    pub async fn request(&mut self, mut payload: serde_json::Value) -> Result<String, Error> {
        let id = format!("{}", self.next_id);
        self.next_id += 1;
        if let Some(object) = payload.as_object_mut() {
            object.insert("id".to_string(), serde_json::json!(id));
        }
        let line = serde_json::to_string(&payload).map_err(|_| Error::TransportClosed)?;
        self.requests.send(line).await.map_err(|_| Error::TransportClosed)?;
        Ok(id)
    }

    /// Submit a prompt. Returns the request id at once. The terminal
    /// `prompt_result { id, status }` frame arrives on the event stream.
    pub async fn prompt(&mut self, request: PromptRequest) -> Result<String, Error> {
        self.request(request.to_value()).await
    }

    /// Take over the event stream. Later requests stay usable.
    pub fn take_events(&mut self) -> mpsc::UnboundedReceiver<Event> {
        let (_, orphan_rx) = mpsc::unbounded_channel();
        std::mem::replace(&mut self.events, orphan_rx)
    }

    /// Join the reader task. Call after the child exits.
    pub async fn join_reader(self) {
        let _ = self.reader_task.await;
    }
}

/// Send stdin lines. The channel closes when the Session drops.
async fn write_loop(mut request_receiver: mpsc::Receiver<String>, mut stdin: ChildStdin) {
    while let Some(line) = request_receiver.recv().await {
        if stdin.write_all(line.as_bytes()).await.is_err() {
            break;
        }
        if stdin.write_all(b"\n").await.is_err() {
            break;
        }
        if stdin.flush().await.is_err() {
            break;
        }
    }
}

/// Read stdout lines, skip banner text, decode, and forward frames.
async fn read_loop(stdout: tokio::process::ChildStdout, event_sender: mpsc::UnboundedSender<Event>) {
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {},
            Err(error) => {
                tracing::warn!(error = %error, "stdout read failed");
                break;
            },
        }
        match decode_line(&line) {
            RawLine::Event(event) => {
                if event_sender.send(*event).is_err() {
                    break;
                }
            },
            RawLine::Skipped(_) => {},
        }
    }
}
