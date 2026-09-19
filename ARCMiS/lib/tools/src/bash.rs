//! `bash` runs one shell command and returns its combined output.
//!
//! The tool runs the command with `bash -c` inside the tool root. Stdout and
//! stderr merge into one text block, cut at 50 KiB. The timeout is seconds
//! with default 300, clamped to the range 1 through 3600. The `pty` flag is
//! accepted but ignored, because this crate allocates no pseudo terminal.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::pin::pin;
use std::process::Stdio;
use std::time::Duration;

/// `bash` runs one shell command inside the tool root and returns its output.
pub struct Bash {
    /// Root directory for the optional relative `cwd` argument.
    pub root: PathBuf,
}

impl rig::tool::Tool for Bash {
    const NAME: &'static str = "bash";
    type Error = rig::tool::ToolExecutionError;
    type Args = BashArgs;
    type Output = rig::tool::ToolOutput;

    fn description(&self) -> String {
        "Run one shell command and return its combined output.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command text"
                },
                "env": {
                    "type": "object",
                    "additionalProperties": { "type": "string" },
                    "description": "Extra environment variables"
                },
                "timeout": {
                    "type": "number",
                    "description": "Timeout in seconds, default 300, range 1 to 3600"
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory relative to the tool root"
                },
                "pty": {
                    "type": "boolean",
                    "description": "Ignored. This crate allocates no pseudo terminal"
                }
            },
            "required": ["command"]
        })
    }

    async fn call(&self, _context: &mut rig::tool::ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let env_map = args.env.unwrap_or_default();
        for key in env_map.keys() {
            if !valid_env_key(key) {
                return Err(rig::tool::ToolExecutionError::invalid_args(format!(
                    "env key '{key}' is not a valid name. Use letters, digits, and underscores."
                )));
            }
        }
        let cwd = match &args.cwd {
            Some(cwd) => {
                crate::util::path::path_sanitize(&self.root, cwd).map_err(rig::tool::ToolExecutionError::other)?
            },
            None => self.root.clone(),
        };
        let seconds = clamp_seconds(args.timeout.unwrap_or(DEFAULT_TIMEOUT));
        let arguments = ["-c".to_owned(), args.command];
        let run = run_captured("bash", &arguments, Some(&cwd), &env_map, seconds).await?;
        let mut text = truncate_output(&run.output);
        if run.output.is_empty() {
            text = "(no output)".to_owned();
        }
        if run.code != 0 {
            text.push_str(&format!("\nCommand exited with code {}", run.code));
        }
        Ok(rig::tool::ToolOutput::text(text))
    }
}

/// Arguments for `bash`.
#[derive(Debug, serde::Deserialize)]
pub struct BashArgs {
    pub command: String,
    pub env: Option<HashMap<String, String>>,
    pub timeout: Option<u64>,
    pub cwd: Option<String>,
    /// Requested terminal mode. Ignored, no pseudo terminal here.
    pub pty: Option<bool>,
}

/// Default timeout in seconds.
const DEFAULT_TIMEOUT: u64 = 300;

/// Output size kept before truncation.
const OUTPUT_LIMIT: usize = 50 * 1024;

/// Check one env key shape. Names start with a letter or an underscore.
fn valid_env_key(key: &str) -> bool {
    let mut characters = key.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {},
        _ => return false,
    }
    characters.all(|next| next.is_ascii_alphanumeric() || next == '_')
}

/// Clamp a timeout to the range 1 through 3600 seconds.
fn clamp_seconds(timeout: u64) -> u64 {
    timeout.clamp(1, 3600)
}

/// Spawn one program with a watchdog timeout and capture its output.
async fn run_captured(
    program: &str,
    arguments: &[String],
    cwd: Option<&Path>,
    env_map: &HashMap<String, String>,
    seconds: u64,
) -> Result<Captured, rig::tool::ToolExecutionError> {
    let mut command = tokio::process::Command::new(program);
    command.args(arguments).envs(env_map).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let mut child = command
        .spawn()
        .map_err(|error| rig::tool::ToolExecutionError::other(format!("{program} failed to start: {error}")))?;
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
            return Err(rig::tool::ToolExecutionError::timeout(format!("{program} timed out after {seconds} seconds")));
        },
    };
    let code = status
        .map_err(|error| rig::tool::ToolExecutionError::other(format!("{program} status failed: {error}")))?
        .code()
        .unwrap_or(-1);
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

/// Cut output at 50 KiB and mark the cut.
fn truncate_output(output: &str) -> String {
    if output.len() <= OUTPUT_LIMIT {
        return output.to_owned();
    }
    let cut = String::from_utf8_lossy(&output.as_bytes()[..OUTPUT_LIMIT]).into_owned();
    format!("{cut}\n[output truncated at 50 KiB]")
}

/// Captured result of one process run.
struct Captured {
    code: i32,
    output: String,
}
