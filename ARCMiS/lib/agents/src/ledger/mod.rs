//! The ledger agent namespace. `Ledger::build` wires the manager and the
//! worker, `Ledger::run` executes one task. The worker tool routes through
//! `Worker::run`, so every delegation carries its own fresh turn and
//! retry budget instead of draining the manager's.

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
    /// and tool call; the worker runs under the no-op hook. `budgets`
    /// carries the manager and worker turn budgets plus the shared retry
    /// count. The output root is the config's output dir.
    pub fn build<C>(
        client: &C,
        config: &Config,
        provider: &Provider,
        hook: impl AgentHook + Clone + 'static,
        budgets: crate::util::config::LedgerBudgets,
    ) -> Agent
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
        // it through the worker tool on every call, and every delegation
        // starts with the full worker budget.
        let worker = worker::Worker::build(client, config, provider, write, bash);
        let worker_tool = LedgerManager::worker_tool(worker, budgets.worker_turns, budgets.retries);
        LedgerManager::build(client, config, provider, worker_tool, hook)
    }

    /// Run the ledger manager over one task. `max_turns` bounds the
    /// model-call budget. `max_retries` bounds whole-task retries. Returns
    /// the structured result artifact. The model must deliver it through
    /// the output-tool call.
    pub async fn run(
        agent: &Agent,
        task: &MonolithRequest,
        max_turns: usize,
        max_retries: u32,
    ) -> anyhow::Result<LedgerResponse> {
        crate::util::task::task(agent, task, max_turns, max_retries).await
    }
}
