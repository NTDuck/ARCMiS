//! `config` carries the spawn knobs for one omp RPC child.

use crate::frame_payloads::StreamingBehavior;
use serde_json::Value;
use std::path::PathBuf;

/// Advertised frame limits from the `ready` frame, with sane defaults.
#[derive(Debug, Clone, Copy)]
pub struct FrameLimits {
    pub max_frame_bytes: u64,
    pub max_reassembled_frame_bytes: u64,
}

impl Default for FrameLimits {
    fn default() -> Self {
        Self {
            max_frame_bytes: 1_048_576,
            max_reassembled_frame_bytes: 67_108_864,
        }
    }
}

/// Knobs for the child process. The caller owns every value.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub program: String,
    pub model: Option<String>,
    pub approval_mode: String,
    pub extra_args: Vec<String>,
    pub working_dir: PathBuf,
    pub env: Vec<(String, String)>,
    pub frame_limits: FrameLimits,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            program: "omp".to_string(),
            model: None,
            approval_mode: "yolo".to_string(),
            extra_args: Vec::new(),
            // WHY a literal: the config rule bans environment reads in library
            // code. `Command::current_dir(".")` inherits the parent cwd at
            // spawn time, so the behavior matches a plain shell launch.
            working_dir: PathBuf::from("."),
            env: Vec::new(),
            frame_limits: FrameLimits::default(),
        }
    }
}

impl SessionConfig {
    /// Build the child argv: mode, model, approval mode, passthrough args.
    ///
    /// WHY `--model`: omp 18.6.1 selects the session model with
    /// `--model=<value>` (fuzzy match, `provider/model`). The form comes
    /// from `omp --help`. No RPC-level model message exists.
    pub fn argv(&self) -> Vec<String> {
        let mut argv = vec!["--mode".to_string(), "rpc".to_string()];
        if let Some(model) = &self.model {
            argv.push(format!("--model={model}"));
        }
        argv.push(format!("--approval-mode={}", self.approval_mode));
        argv.extend(self.extra_args.iter().cloned());
        argv
    }
}

/// Body of a `prompt` request.
#[derive(Debug, Clone)]
pub struct PromptRequest {
    pub message: String,
    pub streaming_behavior: Option<StreamingBehavior>,
}

impl PromptRequest {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            streaming_behavior: None,
        }
    }

    /// Serialize for the wire. `streamingBehavior: "steer"` keeps a later
    /// prompt from queuing behind the running turn.
    pub fn to_value(&self) -> Value {
        let mut value = serde_json::json!({ "type": "prompt", "message": self.message });
        if let Some(streaming_behavior) = &self.streaming_behavior {
            value["streamingBehavior"] = serde_json::json!(streaming_behavior);
        }
        value
    }
}
