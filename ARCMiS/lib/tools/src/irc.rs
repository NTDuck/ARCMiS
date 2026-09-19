//! `irc` lists peers and delivers peer messages.
//!
//! The peer registry lives on the tool struct. `send` appends the message to
//! the peer inbox and returns a delivery summary. An unknown peer yields a
//! not-found style result text, not an error. There is no LLM reply loop in
//! this pass. A later pass grows replies and await handling.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

/// `irc` lists peers and delivers messages into peer inboxes.
pub struct Irc {
    /// Peer registry keyed by peer name.
    pub peers: Arc<Mutex<BTreeMap<String, Peer>>>,
}

/// One known peer with its live status and pending messages.
#[derive(Debug, Clone, Default)]
pub struct Peer {
    /// Peer name.
    pub name: String,
    /// Status string, for example "running", "idle", or "parked".
    pub status: String,
    /// Messages delivered to this peer, oldest first.
    pub inbox: Vec<String>,
}

impl rig::tool::Tool for Irc {
    const NAME: &'static str = "irc";
    type Error = rig::tool::ToolExecutionError;
    type Args = IrcArgs;
    type Output = rig::tool::ToolOutput;

    fn description(&self) -> String {
        "List peers or deliver one message into a peer inbox.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "op": { "type": "string", "enum": ["list", "send"], "description": "Operation to run" },
                "to": { "type": "string", "description": "Peer name for send" },
                "message": { "type": "string", "description": "Message text for send" },
                "awaitReply": { "type": "boolean", "description": "Unused in this pass. No reply loop exists yet." }
            },
            "required": ["op"]
        })
    }

    async fn call(&self, _context: &mut rig::tool::ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let text = match args.op.as_str() {
            "list" => list_peers(&self.peers)?,
            "send" => send_message(&self.peers, &args)?,
            other => {
                return Err(rig::tool::ToolExecutionError::invalid_args(format!(
                    "unknown op \"{other}\". Use \"list\" or \"send\"."
                )))
            },
        };
        Ok(rig::tool::ToolOutput::text(text))
    }
}

/// Arguments for `irc`.
#[derive(Debug, serde::Deserialize)]
pub struct IrcArgs {
    pub op: String,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub await_reply: Option<bool>,
}

/// Render every peer with its status and inbox size.
fn list_peers(peers: &Mutex<BTreeMap<String, Peer>>) -> Result<String, rig::tool::ToolExecutionError> {
    let registry = lock_peers(peers)?;
    if registry.is_empty() {
        return Ok(String::from("no peers registered"));
    }
    let rows = registry
        .values()
        .map(|peer| format!("{} ({}), inbox {}", peer.name, peer.status, peer.inbox.len()))
        .collect::<Vec<_>>();
    Ok(rows.join("\n"))
}

/// Append the message to one peer inbox and return the delivery summary.
fn send_message(
    peers: &Mutex<BTreeMap<String, Peer>>,
    args: &IrcArgs,
) -> Result<String, rig::tool::ToolExecutionError> {
    let peer_name = args.to.as_deref().unwrap_or_default();
    if peer_name.is_empty() {
        return Err(rig::tool::ToolExecutionError::invalid_args("send needs a \"to\" peer name"));
    }
    if args.message.as_deref().unwrap_or_default().is_empty() {
        return Err(rig::tool::ToolExecutionError::invalid_args("send needs a non-empty \"message\""));
    }
    let message_text = args.message.clone().unwrap_or_default();
    let mut registry = lock_peers(peers)?;
    let peer = registry.get_mut(peer_name);
    match peer {
        Some(peer) => {
            peer.inbox.push(message_text);
            let inbox_len = peer.inbox.len();
            Ok(format!("delivered to {peer_name}; inbox now holds {inbox_len} message(s)"))
        },
        None => Ok(format!("notFound: no peer named \"{peer_name}\". Run op \"list\" to see the peers.")),
    }
}

/// Lock the peer registry. A poisoned lock returns an execution error.
fn lock_peers(
    peers: &Mutex<BTreeMap<String, Peer>>,
) -> Result<MutexGuard<'_, BTreeMap<String, Peer>>, rig::tool::ToolExecutionError> {
    peers.lock().map_err(|error| rig::tool::ToolExecutionError::other(format!("peer registry lock failed: {error}")))
}
