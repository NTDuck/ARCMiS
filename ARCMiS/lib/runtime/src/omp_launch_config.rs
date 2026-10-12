//! Knobs for the omp child process.
//!
//! The caller owns every value.

use std::path::PathBuf;

use omp_rpc::SubagentSubscriptionLevel;

/// Knobs for the omp child process. The caller owns every value.
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
