//! `transport` spawns one `omp --mode rpc` child and moves frames over channels.

use crate::frame::{decode_line, DecodedLine, Reassembler, ServerFrame};
use anyhow::Context;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Knobs for the child process. The binary bubbles them from its config.
#[derive(Debug, Clone)]
pub struct SpawnConfig {
    pub omp_binary: String,
    pub omp_arguments: Vec<String>,
    pub working_dir: PathBuf,
    pub approval_mode: String,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            omp_binary: "omp".to_string(),
            omp_arguments: vec!["--mode".to_string(), "rpc".to_string()],
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            approval_mode: "yolo".to_string(),
        }
    }
}

/// One live RPC child: request writer, event reader, and the process handle.
pub struct Transport {
    pub requests: mpsc::Sender<String>,
    events: mpsc::Receiver<ServerFrame>,
    pub child: tokio::process::Child,
    reader_task: JoinHandle<()>,
}

impl Transport {
    /// Start the child. Frames land on an unbounded channel.
    pub fn spawn(config: SpawnConfig) -> anyhow::Result<Self> {
        let mut child = tokio::process::Command::new(&config.omp_binary)
            .args(&config.omp_arguments)
            .arg(format!("--approval-mode={}", config.approval_mode))
            .current_dir(&config.working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("spawn {} in {}", config.omp_binary, config.working_dir.display()))?;
        let stdin = child.stdin.take().context("child stdin missing")?;
        let stdout = child.stdout.take().context("child stdout missing")?;
        let (request_sender, request_receiver) = mpsc::channel::<String>(64);
        let (event_sender, event_receiver) = mpsc::channel::<ServerFrame>(4096);
        let writer_task = tokio::spawn(write_loop(request_receiver, stdin));
        let reader_task = tokio::spawn(read_loop(stdout, event_sender));
        std::mem::drop(writer_task);
        Ok(Self {
            requests: request_sender,
            events: event_receiver,
            child,
            reader_task,
        })
    }

    /// Detach the event stream. Later client calls stay usable.
    pub fn take_events(&mut self) -> mpsc::Receiver<ServerFrame> {
        let (_, orphan) = mpsc::channel(1);
        std::mem::replace(&mut self.events, orphan)
    }

    /// Read one frame with a deadline. Used before the mirror loop starts.
    pub async fn read_event_before(&mut self, deadline: tokio::time::Instant) -> anyhow::Result<ServerFrame> {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        tokio::time::timeout(remaining, self.events.recv())
            .await
            .context("timed out waiting for the next frame")?
            .context("event stream ended")
    }

    /// Readiness gate: first `ready` frame with its advertised limits.
    pub async fn ready(&mut self) -> anyhow::Result<crate::frame::ReadyFrame> {
        while let Some(frame) = self.events.recv().await {
            if let ServerFrame::Ready(ready) = frame {
                return Ok(ready);
            }
        }
        anyhow::bail!("child exited before ready frame")
    }
    /// Join the reader task. Call after the child exits to surface read errors.
    pub async fn join_reader(self) {
        let _ = self.reader_task.await;
    }
}

async fn write_loop(mut request_receiver: mpsc::Receiver<String>, mut stdin: ChildStdin) {
    while let Some(request) = request_receiver.recv().await {
        let line = request.trim_end().to_string();
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

async fn read_loop(stdout: ChildStdout, event_sender: mpsc::Sender<ServerFrame>) {
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let mut reassembler = Reassembler::default();
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
        match decode_line(&line, &mut reassembler) {
            Ok(DecodedLine::Frame(frame)) => {
                if event_sender.send(*frame).await.is_err() {
                    break;
                }
            },
            Ok(DecodedLine::ChunkPart) | Ok(DecodedLine::Skipped(_)) => {},
            Err(error) => {
                tracing::warn!(error = %error, "frame decode failed");
            },
        }
    }
}
