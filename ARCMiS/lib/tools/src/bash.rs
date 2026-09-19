//! `bash` runs one shell command and returns its combined output.
//!
//! The tool runs the command with `bash -c` inside the tool root. Stdout and
//! stderr merge into one text block, cut at 50 KiB. The timeout is seconds
//! with default 300, clamped to the range 1 through 3600. The `pty` flag is
//! accepted but ignored, because this crate allocates no pseudo terminal.

use std::collections::HashMap;
use std::path::PathBuf;

use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};

use crate::util::proc::{capture, clamp_seconds, truncate_output};
use serde::Deserialize;

/// `bash` runs one shell command inside the tool root and returns its output.
pub struct Bash {
    /// Root directory for the optional relative `cwd` argument.
    pub root: PathBuf,
}

impl Tool for Bash {
    const NAME: &'static str = "bash";
    type Error = ToolExecutionError;
    type Args = BashArgs;
    type Output = ToolOutput;

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

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let env_map = args.env.unwrap_or_default();
        for key in env_map.keys() {
            if !valid_env_key(key) {
                return Err(ToolExecutionError::invalid_args(format!(
                    "env key '{key}' is not a valid name. Use letters, digits, and underscores."
                )));
            }
        }
        let cwd = match &args.cwd {
            Some(cwd) => crate::util::path::path_sanitize(&self.root, cwd).map_err(ToolExecutionError::other)?,
            None => self.root.clone(),
        };
        let seconds = clamp_seconds(args.timeout.unwrap_or(DEFAULT_TIMEOUT));
        let arguments = ["-c".to_owned(), args.command];
        let run = capture("bash", &arguments, Some(&cwd), Some(&env_map), seconds).await?;
        let mut text = truncate_output(&run.output);
        if run.output.is_empty() {
            text = "(no output)".to_owned();
        }
        if run.code != 0 {
            text.push_str(&format!("\nCommand exited with code {}", run.code));
        }
        Ok(ToolOutput::text(text))
    }
}

/// Arguments for `bash`.
#[derive(Debug, Deserialize)]
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

/// Check one env key shape. Names start with a letter or an underscore.
fn valid_env_key(key: &str) -> bool {
    let mut characters = key.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {},
        _ => return false,
    }
    characters.all(|next| next.is_ascii_alphanumeric() || next == '_')
}
