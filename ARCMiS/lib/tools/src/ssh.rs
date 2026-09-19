//! `ssh` runs one command on a remote host over `ssh`.
//!
//! Host settings come from `.omp/ssh.json` in the tool root, or from
//! `~/.omp/agent/ssh.json` in the home directory. The file maps a host key to
//! a record with `hostname`, `user`, `port`, and optional `key` fields.
//! The tool builds one `ssh` command line and runs it without a terminal.
//! A non-zero remote exit is a text notice, not an error.

use std::collections::BTreeMap;
use std::env::var;
use std::fs::read_to_string;
use std::path::Path;
use std::path::PathBuf;

use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};

use crate::util::proc::{capture, clamp_seconds};

/// `ssh` runs one command on a configured remote host and returns its output.
pub struct Ssh {
    /// Root directory that holds the optional `.omp/ssh.json` host file.
    pub root: PathBuf,
}

impl Tool for Ssh {
    const NAME: &'static str = "ssh";
    type Error = ToolExecutionError;
    type Args = SshArgs;
    type Output = ToolOutput;

    fn description(&self) -> String {
        "Run one command on a configured remote host over ssh.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "host": {
                    "type": "string",
                    "description": "Host key from the ssh host file"
                },
                "command": {
                    "type": "string",
                    "description": "Command text for the remote shell"
                },
                "cwd": {
                    "type": "string",
                    "description": "Remote directory to run the command in"
                },
                "timeout": {
                    "type": "number",
                    "description": "Timeout in seconds, default 60, range 1 to 3600"
                }
            },
            "required": ["host", "command"]
        })
    }

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let hosts = load_hosts(&self.root)?;
        let record = match hosts.get(&args.host) {
            Some(record) => record.clone(),
            None => {
                let mut names = hosts.keys().cloned().collect::<Vec<_>>();
                names.sort_unstable();
                let listed = names.join(", ");
                return Err(ToolExecutionError::not_found(format!(
                    "host '{}' is not configured. Available hosts: {}",
                    args.host,
                    if listed.is_empty() {
                        "(none)"
                    } else {
                        &listed
                    }
                )));
            },
        };
        let seconds = clamp_seconds(args.timeout.unwrap_or(DEFAULT_TIMEOUT));
        let remote = match &args.cwd {
            Some(cwd) => format!("cd {cwd:?} && {}", args.command),
            None => args.command.clone(),
        };
        let ssh_line = build_ssh(&record, &remote);
        let (program, arguments) =
            ssh_line.split_first().ok_or_else(|| ToolExecutionError::other("ssh command line is empty".to_owned()))?;
        let run = capture(program, arguments, None, None, seconds).await?;
        let mut text = run.output;
        if text.is_empty() {
            text = "(no output)".to_owned();
        }
        if run.code != 0 {
            text.push_str(&format!("\nRemote command exited with code {}", run.code));
        }
        Ok(ToolOutput::text(text))
    }
}

/// Arguments for `ssh`.
#[derive(Debug, serde::Deserialize)]
pub struct SshArgs {
    pub host: String,
    pub command: String,
    pub cwd: Option<String>,
    pub timeout: Option<u64>,
}

/// Default timeout in seconds.
const DEFAULT_TIMEOUT: u64 = 60;

/// Connection keep-alive window for the control socket.
const CONTROL_PERSIST: &str = "3600";

/// One host record of the ssh host file.
#[derive(Debug, serde::Deserialize, Clone)]
pub struct SshHost {
    pub hostname: String,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub key: Option<String>,
}

/// Load host records from the tool root, or from the home directory.
fn load_hosts(root: &Path) -> Result<BTreeMap<String, SshHost>, ToolExecutionError> {
    let local = root.join(".omp").join("ssh.json");
    let path = if local.exists() {
        local
    } else {
        let home = var("HOME").unwrap_or_default();
        if home.is_empty() {
            return Ok(BTreeMap::new());
        }
        PathBuf::from(home).join(".omp").join("agent").join("ssh.json")
    };
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = read_to_string(&path).map_err(|error| {
        ToolExecutionError::other(format!("ssh host file {} is not readable: {error}", path.display()))
    })?;
    serde_json::from_str(&text).map_err(|error| {
        ToolExecutionError::other(format!("ssh host file {} is not valid JSON: {error}", path.display()))
    })
}

/// Build the ssh command line for one host record and remote command.
fn build_ssh(record: &SshHost, remote: &str) -> Vec<String> {
    let mut line = vec![
        "ssh".to_owned(),
        "-o".to_owned(),
        "BatchMode=yes".to_owned(),
        "-o".to_owned(),
        "StrictHostKeyChecking=accept-new".to_owned(),
        "-o".to_owned(),
        format!("ControlPersist={CONTROL_PERSIST}"),
    ];
    if let Some(key) = &record.key {
        line.push("-i".to_owned());
        line.push(expand_home(key));
    }
    if let Some(port) = record.port {
        line.push("-p".to_owned());
        line.push(format!("{port}"));
    }
    let target = match &record.user {
        Some(user) => format!("{}@{}", user, record.hostname),
        None => record.hostname.clone(),
    };
    line.push(target);
    line.push("--".to_owned());
    line.push(remote.to_owned());
    line
}

/// Expand a leading `~` in a key path to the home directory.
fn expand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = var("HOME").unwrap_or_default();
        if !home.is_empty() {
            return format!("{home}/{rest}");
        }
    }
    path.to_owned()
}
