# 0035. oh-my-pi-omp Rust SDK as the agent transport

- **Date:** 2026-10-12
- **Status:** accepted
- **Supersedes:** [0033](0033-omprpc-stdio-client-over-crates-io-omp-rpc.md)
- **Related:** [0030](0030-omprpc-transport-replaces-rig.md), [0034](0034-mas-core-transport-seam.md)

## Context

ADR 0033 rejected the crates.io `omp-rpc` 0.1.0 as an agent transport.
That evaluation hit the wrong project. The crates.io package belongs
to `github.com/can1357/omp`. Its repository link, its description
("gRPC transport, handshake, health, TLS, and Unix-socket plumbing
for omp"), and its gRPC service set are that project's daemon
plumbing. It has no agent-session service.

The local `omp` binary builds from `github.com/xoannova/oh-my-pi-omp`.
The authoritative client for its `omp --mode rpc` stdio protocol is
the Rust SDK at `sdk/rust/omp-rpc` in that repository. Its manifest
declares `name = "omp-rpc"`, `version = "0.1.0"`, and
`publish = false`, so xoannova never published it to crates.io. The
identical name and version of the unrelated crates.io crate make the
two easy to confuse. Only the git source is the real client.

## Decision

Adopt the xoannova SDK as the sole agent transport client. Declare it
in the workspace as
`omp-rpc = { git = "https://github.com/xoannova/oh-my-pi-omp", rev = "40e9368ef0458fd9073329cdff4174895f91bc6b" }`.
Retire `ARCMiS-omprpc`. `mas_core::transport::OmpTransport` wraps one
`omp_rpc::Client`. The launch binary stays a parameter of
`OmpLaunchConfig`. The `ompl` host wrapper is a call-site value, never
a library constant.

## Evidence

The SDK public session surface, verbatim from the pinned rev:

```rust
// client.rs
pub fn spawn(process: Process, options: ClientOptions) -> Result<(Self, Receiver<Event>), Error>
pub fn call<C: Command>(&self, command: &C) -> Result<C::Output, Error>
pub fn send(&self, frame: &RpcInbound) -> Result<(), Error>
pub fn prompt_and_wait(&self, prompt: &PromptCommand, timeout: Duration) -> Result<PromptTurn, Error>
pub fn set_custom_tools(&self, tools: Vec<HostTool>) -> Result<Vec<String>, Error>
pub fn set_host_uris(&self, uris: Vec<HostUri>) -> Result<Vec<String>, Error>
pub fn get_messages(&self) -> Result<Vec<AgentMessage>, Error>
pub fn close(mut self) -> Result<(), Error>
pub fn pid(&self) -> Option<u32>
pub fn ready(&self) -> &ReadyEvent
pub fn protocol_version(&self) -> u32
pub fn set_default_timeout(&mut self, timeout: Duration)
pub struct ClientOptions {
    pub tools: Vec<HostTool>,
    pub uris: Vec<HostUri>,
    pub ready_timeout: Duration,
    pub default_timeout: Duration,
}
pub enum Error {
    Io(io::Error),
    Json(serde_json::Error),
    Command { command: String, error: String, code: Option<String> },
    Timeout { command: String },
    Closed,
    Protocol(String),
    InvalidArgument(String),
}
pub enum Event {
    Notification(RpcNotification),
    UnmatchedResponse(RpcResponse),
    Undecodable { raw: Value, error: String },
}
// wire.rs
pub trait Command: Serialize {
    const NAME: &'static str;
    const TIMEOUT_MS: Option<u64>;
    type Output;
    fn decode(data: Option<Value>) -> Result<Self::Output, serde_json::Error>;
}
pub struct PromptCommand {
    pub message: String,
    pub images: Option<Vec<ImageContent>>,
    pub streaming_behavior: Option<StreamingBehavior>,
}
pub struct SetSubagentSubscriptionCommand {
    pub level: SubagentSubscriptionLevel,
}
```

`Client::spawn` takes the caller's `std::process::Command`, so the
binary name, arguments, environment, and working directory stay at
the call site. On Unix the child leads its own process group.
Teardown signals that group. The handshake runs inside `connect`:
wait for `ready`, negotiate protocol v2 when the server advertises it
with the standard limits, then register tools and URIs.

Protocol surface covered against the wire census: the `ready`
handshake, `negotiate_protocol`, `set_subagent_subscription`,
`prompt` / `prompt_result`, and subagent lifecycle, progress, and
event frames with routing by `payload.agent`. The transport sends the
subscription after spawn at the configured level. Without it the
subagent stream stays silent and nothing errors. The SDK reader
treats a non-JSON stdout line as a protocol failure. The installed
binary sends a clean `ready` first (verified live on 2026-10-12), so
the retired client's banner skip is unnecessary.

One surface needs a host-side bridge. The SDK assigns request ids
itself (`req_N`) and never returns them. `PromptAck` carries only
`agentInvoked`. `OmpTransport` therefore issues numeric tokens,
re-keys each terminal `prompt_result` to the token `send_prompt`
returned, and relies on the session processing prompts serially, so
completion order equals submission order. The SDK's Python and Go
clients expose caller-visible ids (`prompt() -> str`,
`WithRequestID`). The Rust client lacks that parity.

## Consequences

- `ARCMiS-omprpc` is retired. Its wire vocabulary persists in
  `mas_core::frames`, which keeps the 18.6.1 census shapes.
- The crates.io `omp-rpc` stays out of the dependency graph. Its name
  collides with the xoannova crate. A registry bump must not read as
  an SDK upgrade.
- The launch binary stays configurable. The `ompl` wrapper sits in
  the `program` field at every call site.
- Re-evaluate this record if the SDK ships a caller-visible request
  id. The token re-key then disappears.
- This record supersedes ADR 0033. That record evaluated the
  can1357/omp crate, not this SDK.
