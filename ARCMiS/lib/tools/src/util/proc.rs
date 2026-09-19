//! Process capture helpers shared by the runtime tools.
//!
//! One watchdog spawn covers `bash`, `eval`, and `ssh`: stdout and stderr
//! merge into one text block under a timeout, and a timeout kill is
//! distinguishable from a spawn failure.

use rig::tool::ToolExecutionError;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::pin::pin;
use std::process::Stdio;
use std::time::Duration;

/// Output size kept before truncation.
pub const OUTPUT_LIMIT: usize = 50 * 1024;

/// Captured result of one process run.
pub struct Captured {
    /// Process exit code, or `-1` when the wait failed.
    pub code: i32,
    /// Combined stdout and stderr text.
    pub output: String,
}

/// One capture failure with its kind and message.
#[derive(Debug)]
pub enum ProcError {
    /// The program did not start.
    Spawn(String),
    /// The watchdog elapsed before the program exited.
    Timeout(String),
    /// The exit status itself failed to resolve.
    Status(String),
}

impl ProcError {
    /// Map the failure to the tool execution error. A timeout keeps its
    /// kind, every other failure maps to `other`.
    pub fn into_tool_error(self) -> ToolExecutionError {
        match self {
            ProcError::Timeout(message) => ToolExecutionError::timeout(message),
            ProcError::Spawn(message) | ProcError::Status(message) => ToolExecutionError::other(message),
        }
    }
}

impl fmt::Display for ProcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcError::Spawn(message) | ProcError::Timeout(message) | ProcError::Status(message) => {
                formatter.write_str(message)
            },
        }
    }
}

impl From<ProcError> for ToolExecutionError {
    fn from(error: ProcError) -> Self {
        error.into_tool_error()
    }
}

/// Spawn one program with a watchdog timeout and capture its output.
///
/// `envs` adds environment entries when present. `cwd` sets the working
/// directory when present. A timeout kills the child and reports
/// [`ProcError::Timeout`].
pub async fn capture(
    program: &str,
    arguments: &[String],
    cwd: Option<&Path>,
    envs: Option<&HashMap<String, String>>,
    seconds: u64,
) -> Result<Captured, ProcError> {
    let mut command = tokio::process::Command::new(program);
    command.args(arguments).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    if let Some(envs) = envs {
        command.envs(envs);
    }
    let mut child = command.spawn().map_err(|error| ProcError::Spawn(format!("{program} failed to start: {error}")))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let capture = tokio::time::timeout(
        Duration::from_secs(seconds),
        pin!(async {
            let (out_bytes, err_bytes, status) = tokio::join!(drain(stdout), drain(stderr), child.wait());
            (out_bytes, err_bytes, status)
        }),
    )
    .await;
    let (stdout_bytes, stderr_bytes, status) = match capture {
        Ok(parts) => parts,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(ProcError::Timeout(format!("{program} timed out after {seconds} seconds")));
        },
    };
    let code = match status {
        Ok(status) => status.code().unwrap_or(-1),
        Err(error) => return Err(ProcError::Status(format!("{program} status failed: {error}"))),
    };
    let mut output = String::from_utf8_lossy(&stdout_bytes).into_owned();
    output.push_str(&String::from_utf8_lossy(&stderr_bytes));
    Ok(Captured {
        code,
        output,
    })
}

/// Read one pipe to the end and return its bytes.
async fn drain<R>(pipe: Option<R>) -> Vec<u8>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut pipe = match pipe {
        Some(pipe) => pipe,
        None => return Vec::new(),
    };
    let mut bytes = Vec::new();
    let _ = tokio::io::AsyncReadExt::read_to_end(&mut pipe, &mut bytes).await;
    bytes
}

/// Clamp a timeout to the range 1 through 3600 seconds.
pub fn clamp_seconds(timeout: u64) -> u64 {
    timeout.clamp(1, 3600)
}

/// Cut output at 50 KiB and mark the cut.
pub fn truncate_output(output: &str) -> String {
    if output.len() <= OUTPUT_LIMIT {
        return output.to_owned();
    }
    let cut = String::from_utf8_lossy(&output.as_bytes()[..OUTPUT_LIMIT]).into_owned();
    format!("{cut}\n[output truncated at 50 KiB]")
}
