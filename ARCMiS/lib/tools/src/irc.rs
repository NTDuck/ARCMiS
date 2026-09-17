//! `irc` lists peers and delivers peer messages.
//!
//! The peer registry lives on the tool struct. `send` appends the message to
//! the peer inbox and returns a delivery summary. An unknown peer yields a
//! not-found style result text, not an error. There is no LLM reply loop in
//! this pass. A later pass grows replies and await handling.

/// `irc` lists peers and delivers messages into peer inboxes.
pub struct Irc {
    /// Peer registry keyed by peer name.
    pub peers: ::std::sync::Arc<::std::sync::Mutex<::std::collections::BTreeMap<::std::string::String, Peer>>>,
}

/// One known peer with its live status and pending messages.
#[derive(::core::fmt::Debug, ::core::clone::Clone, ::core::default::Default)]
pub struct Peer {
    /// Peer name.
    pub name: ::std::string::String,
    /// Status string, for example "running", "idle", or "parked".
    pub status: ::std::string::String,
    /// Messages delivered to this peer, oldest first.
    pub inbox: ::std::vec::Vec<::std::string::String>,
}

impl ::rig::tool::Tool for Irc {
    const NAME: &'static str = "irc";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = IrcArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> ::std::string::String {
        "List peers or deliver one message into a peer inbox.".to_owned()
    }

    fn parameters(&self) -> ::serde_json::Value {
        ::serde_json::json!({
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

    async fn call(
        &self,
        _context: &mut ::rig::tool::ToolContext,
        args: Self::Args,
    ) -> ::core::result::Result<Self::Output, Self::Error> {
        let text = match args.op.as_str() {
            "list" => list_peers(&self.peers)?,
            "send" => send_message(&self.peers, &args)?,
            other => {
                return ::core::result::Result::Err(::rig::tool::ToolExecutionError::invalid_args(::std::format!(
                    "unknown op \"{other}\". Use \"list\" or \"send\"."
                )))
            },
        };
        ::core::result::Result::Ok(::rig::tool::ToolOutput::text(text))
    }
}

/// Arguments for `irc`.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
pub struct IrcArgs {
    pub op: ::std::string::String,
    #[serde(default)]
    pub to: ::core::option::Option<::std::string::String>,
    #[serde(default)]
    pub message: ::core::option::Option<::std::string::String>,
    #[serde(default)]
    pub await_reply: ::core::option::Option<bool>,
}

/// Render every peer with its status and inbox size.
fn list_peers(
    peers: &::std::sync::Mutex<::std::collections::BTreeMap<::std::string::String, Peer>>,
) -> ::core::result::Result<::std::string::String, ::rig::tool::ToolExecutionError> {
    let registry = lock_peers(peers)?;
    if registry.is_empty() {
        return ::core::result::Result::Ok(::std::string::String::from("no peers registered"));
    }
    let rows = registry
        .values()
        .map(|peer| ::std::format!("{} ({}), inbox {}", peer.name, peer.status, peer.inbox.len()))
        .collect::<::std::vec::Vec<_>>();
    ::core::result::Result::Ok(rows.join("\n"))
}

/// Append the message to one peer inbox and return the delivery summary.
fn send_message(
    peers: &::std::sync::Mutex<::std::collections::BTreeMap<::std::string::String, Peer>>,
    args: &IrcArgs,
) -> ::core::result::Result<::std::string::String, ::rig::tool::ToolExecutionError> {
    let peer_name = args.to.as_deref().unwrap_or_default();
    if peer_name.is_empty() {
        return ::core::result::Result::Err(::rig::tool::ToolExecutionError::invalid_args(
            "send needs a \"to\" peer name",
        ));
    }
    if args.message.as_deref().unwrap_or_default().is_empty() {
        return ::core::result::Result::Err(::rig::tool::ToolExecutionError::invalid_args(
            "send needs a non-empty \"message\"",
        ));
    }
    let message_text = args.message.clone().unwrap_or_default();
    let mut registry = lock_peers(peers)?;
    let peer = registry.get_mut(peer_name);
    match peer {
        ::core::option::Option::Some(peer) => {
            peer.inbox.push(message_text);
            let inbox_len = peer.inbox.len();
            ::core::result::Result::Ok(::std::format!(
                "delivered to {peer_name}; inbox now holds {inbox_len} message(s)"
            ))
        },
        ::core::option::Option::None => ::core::result::Result::Ok(::std::format!(
            "notFound: no peer named \"{peer_name}\". Run op \"list\" to see the peers."
        )),
    }
}

/// Lock the peer registry. A poisoned lock returns an execution error.
fn lock_peers(
    peers: &::std::sync::Mutex<::std::collections::BTreeMap<::std::string::String, Peer>>,
) -> ::core::result::Result<
    ::std::sync::MutexGuard<'_, ::std::collections::BTreeMap<::std::string::String, Peer>>,
    ::rig::tool::ToolExecutionError,
> {
    peers
        .lock()
        .map_err(|error| ::rig::tool::ToolExecutionError::other(::std::format!("peer registry lock failed: {error}")))
}
