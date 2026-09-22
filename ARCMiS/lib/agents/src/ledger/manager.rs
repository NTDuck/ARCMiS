//! The ledger manager. Zero-shot self-orchestration: it plans first, then
//! delegates one task per call through the worker tool, re-curates the task
//! list after each round from the shared workspace files, and decides when
//! to stop. The paper's v2 scaffold (arXiv:2608.26480 section 3.1).

use crate::util::config::Config;
use crate::util::provider::Provider;
use rig::agent::{Agent, AgentHook, OutputMode};
use rig::client::AgentClientExt;
use rig::tool::DynamicTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Preamble for the ledger manager. The v2 control flow: write the plan
/// and the seed task list, brainstorm through one worker, then run the
/// manage loop. Between rounds the manager re-reads the workspace files
/// (plan.md, tasks.json, notes.md) and re-curates the task list: merge
/// duplicates, drop done items, add only genuinely new sub-tasks. The
/// manager never writes files itself; the worker tool is its only lever
/// on the workspace.
const MANAGER_PREAMBLE: &str = "\
You manage the translation of the codebase in the task. The workspace \
is your shared ledger: plan.md holds the plan, tasks.json holds the \
task list, notes.md holds accumulated worker findings, and the output \
codebase accumulates in the workspace root.\n\
\n\
Flow:\n\
1. Write plan.md with a short strategy and 3 to 6 seed tasks in \
tasks.json format.\n\
2. Delegate one brainstorm task first: the worker lists the core \
difficulties and candidate approaches before any code.\n\
3. Loop: read the workspace files and the worker reports, re-curate \
the task list (merge duplicates, drop done items, add only genuinely \
new sub-tasks), then delegate the single most valuable next task.\n\
4. Verify each worker round against the build and test state that the \
worker reports. A failed verification overrides any done claim.\n\
5. Stop and report when the translation builds and its tests pass, or \
when no progress remains. Do not write files yourself.";

/// The ledger manager agent namespace. `LedgerManager::build` wires the
/// manager over one worker tool.
pub struct LedgerManager;

impl LedgerManager {
    /// Build the ledger manager. `worker_tool` is the worker agent exposed
    /// as a dynamic tool: the manager's only lever on the workspace. `hook`
    /// observes every manager model call and tool call. All knobs come from
    /// the config.
    pub fn build<C>(
        client: &C,
        config: &Config,
        provider: &Provider,
        worker_tool: DynamicTool,
        hook: impl AgentHook + 'static,
    ) -> Agent
    where
        C: AgentClientExt,
        C::CompletionModel: 'static,
    {
        let mut builder = client
            .agent(&config.run.model)
            .name("ledger_manager")
            .preamble(MANAGER_PREAMBLE)
            .dynamic_tool(worker_tool)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .output_schema::<LedgerResponse>()
            .output_mode(OutputMode::Tool)
            .add_hook(hook);
        if let Some(params) = provider.extra_params(&config.run) {
            builder = builder.additional_params(params);
        }
        builder.build()
    }
}

// Output artifact owned exclusively by this agent. Per the
// artifact-ownership rule, the DTO lives here. Per the Stepdown Rule, it
// sits below the entry functions as a secondary type that serves them. The
// input artifact is the shared [`crate::monolith::MonolithRequest`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LedgerResponse {
    /// Build outcome of the translated codebase: true when the build
    /// succeeds.
    pub compiled: bool,
    /// Fraction of translated tests that pass. `None` when the target has
    /// no test suite.
    pub test_pass_rate: Option<f64>,
    /// One-line summary of the translation approach, for the validator's context.
    pub approach: String,
}
