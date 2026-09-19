//! The ledger manager. Zero-shot self-orchestration: it delegates one task
//! per call through the worker tool, re-curates the task list after each
//! round, and decides when to stop.

use rig::agent::{Agent, OutputMode};
use rig::client::AgentClientExt;

/// Preamble for the ledger manager. The loop: read the workspace, delegate
/// one task per call, re-curate the task list, verify, stop when done.
const MANAGER_PREAMBLE: &str = "\
You manage the translation of the codebase in the task. You have one \
worker tool. Delegate one task per call: give the worker the full \
instructions for one step. The worker writes files and runs builds in \
the shared workspace. After each worker call, re-read the workspace \
state and re-curate the task list: merge duplicates, drop done items, \
add only genuinely new sub-tasks. Decide from the worker reports \
whether to continue, reissue, or stop. Stop and report the final \
result when the translation builds and its tests pass, or when no \
progress remains. Do not write files yourself.";

/// The ledger manager agent namespace. `LedgerManager::build` wires the
/// manager over one worker tool.
pub struct LedgerManager;

impl LedgerManager {
    /// Build the ledger manager. `worker_tool` is the worker agent exposed
    /// as a dynamic tool: the manager's only lever on the workspace. `hook`
    /// observes every manager model call and tool call. All knobs come from
    /// the config.
    pub fn build(
        client: &rig::providers::ollama::Client,
        config: &crate::util::config::Config,
        worker_tool: rig::tool::DynamicTool,
        hook: impl rig::agent::AgentHook + 'static,
    ) -> Agent {
        client
            .agent(&config.run.model)
            .name("ledger_manager")
            .preamble(MANAGER_PREAMBLE)
            .dynamic_tool(worker_tool)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .additional_params(serde_json::json!({
                "num_ctx": config.run.num_ctx,
                "think": config.run.think,
            }))
            .output_schema::<LedgerResponse>()
            .output_mode(OutputMode::Tool)
            .add_hook(hook)
            .build()
    }
}

// Output artifact owned exclusively by this agent. Per the
// artifact-ownership rule, the DTO lives here. Per the Stepdown Rule, it
// sits below the entry functions as a secondary type that serves them. The
// input artifact is the shared [`crate::monolith::MonolithRequest`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct LedgerResponse {
    /// Build outcome of the translated codebase: `pass` when the build
    /// succeeds, `fail` otherwise.
    pub compilation_status: String,
    /// Fraction of translated tests that pass. `None` when the target has
    /// no test suite.
    pub test_pass_rate: Option<f64>,
    /// One-line summary of the translation approach, for the validator's context.
    pub approach: String,
}
