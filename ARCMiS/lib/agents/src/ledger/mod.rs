//! ## The ledger agent
//!
//! The manager-worker scaffold of arXiv:2608.26480. A manager agent
//! decomposes the translation into one task at a time and delegates each
//! task to a worker agent through the worker tool. The shared filesystem
//! workspace is the only coordination channel: the manager never writes
//! files, and the worker never talks to the manager except through its
//! report. The scaffold is zero-shot: no curated demonstrations, and every
//! role runs the same model in a fresh context. The manager re-curates the
//! task list after each round and decides when to stop.
//!
//! The worker is a stripped monolith: one preamble with the working rules,
//! the `write` and `bash` tools, and no output schema. The manager carries
//! the typed [`LedgerResponse`] through `output_schema` with
//! `OutputMode::Tool`.

pub mod manager;
pub mod worker;

pub use manager::{LedgerManager, LedgerResponse};

use crate::monolith::MonolithRequest;
use crate::util::config::Config;
use crate::util::provider::Provider;
use rig::agent::{Agent, AgentHook};
use rig::client::AgentClientExt;
use tools::{Bash, Write};

/// The ledger agent namespace. `Ledger::build` wires the manager and the
/// worker, `Ledger::run` executes one task.
pub struct Ledger;

impl Ledger {
    /// Build the ledger manager. `hook` observes every manager model call
    /// and tool call. All knobs come from the config. The output root is
    /// the config's output dir. The worker shares the same tool state and
    /// runs under the private no-op hook.
    pub fn build<C>(client: &C, config: &Config, provider: &Provider, hook: impl AgentHook + 'static) -> Agent
    where
        C: AgentClientExt,
        C::CompletionModel: 'static,
    {
        // One snapshot store per build. `write` results carry fresh hashline
        // anchors minted from it.
        let snapshots = tools::SnapshotStore::new();
        let write = Write {
            root: config.output.dir.clone(),
            snapshots,
        };
        let bash = Bash {
            root: config.output.dir.clone(),
        };
        // The worker. Fresh context per delegated task: the manager spawns
        // it through the worker tool on every call.
        let worker = worker::Worker::build(client, config, provider, write, bash);
        LedgerManager::build(client, config, provider, worker.into_tool(), hook)
    }

    /// Run the ledger manager over one task. `max_turns` bounds the
    /// model-call budget. Returns the structured result artifact. The
    /// model must deliver it through the output-tool call.
    pub async fn run(agent: &Agent, task: &MonolithRequest, max_turns: usize) -> anyhow::Result<LedgerResponse> {
        crate::util::task::task(agent, task, max_turns).await
    }
}
