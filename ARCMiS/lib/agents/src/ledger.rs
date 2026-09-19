//! ## The ledger agent
//!
//! The manager-worker scaffold of arXiv:2608.26480. A manager agent
//! decomposes the translation into one task at a time and delegates each
//! task to a worker agent through the worker tool. The shared filesystem
//! workspace is the only coordination channel: the manager never writes
//! files, and the worker never talks to the manager except through its
//! report. The scaffold is zero-shot: no curated demonstrations, and every
//! role runs the same model in a fresh context. The manager spawns one
//! fresh-context worker per delegated task through the worker tool.
//!
//! The worker is a stripped monolith: one preamble with the working rules,
//! the `write` and `bash` tools, and no output schema. The manager carries
//! the typed [`LedgerResponse`] through `output_schema` with
//! `OutputMode::Tool`.

use rig::agent::{Agent, OutputMode};
use rig::client::AgentClientExt;
use tools::{Bash, Write};

use crate::monolith::MonolithRequest;

/// Preamble for the ledger worker. Working rules only. The task data
/// travels in the prompt payload.
const WORKER_PREAMBLE: &str = "\
You translate the source codebase given in the task to the target \
language. You own the whole output package in your workspace: plan the \
structure, generate the manifest, and create every file yourself.\n\
\n\
Work rules:\n\
- Write the translated files with the write tool. Build with the bash \
tool. Fix every build error, then run the tests. If tests fail, fix \
the code and repeat.\n\
- Do not weaken or delete a translated test to make it pass.\n\
- Work only inside your workspace. Do not explore outside it.\n\
- When the build and the tests pass, or when you cannot progress \
further, report with the final report.\n\
- A manager coordinates you. Do the single task in the message. Append \
what you did to notes.md in the workspace root.";

/// Preamble for the ledger manager. The loop: read the workspace, delegate
/// one task per call, verify, stop when done.
const MANAGER_PREAMBLE: &str = "\
You manage the translation of the codebase in the task. You have one \
worker tool. Delegate one task per call: give the worker the full \
instructions for one step. The worker writes files and runs builds in \
the shared workspace. After each worker call, decide the next task \
from what the worker reports. Stop and report the final result when \
the translation builds and its tests pass, or when no progress \
remains. Do not write files yourself.";

/// No-op hook for inner workers. The outer hook observes the manager.
#[derive(Clone, Default)]
struct NoopHook;

impl rig::agent::AgentHook for NoopHook {
    async fn on_completion_call(
        &self,
        _ctx: &rig::agent::HookContext,
        _event: rig::agent::CompletionCallEvent<'_>,
    ) -> rig::agent::CompletionCallAction {
        rig::agent::CompletionCallAction::continue_run()
    }

    async fn on_tool_call(
        &self,
        _ctx: &rig::agent::HookContext,
        _event: rig::agent::ToolCall<'_>,
    ) -> rig::agent::ToolCallAction {
        rig::agent::ToolCallAction::run()
    }
}

/// The ledger agent namespace. `Ledger::build` wires the manager and the
/// worker, `Ledger::run` executes one task.
pub struct Ledger;

impl Ledger {
    /// Build the ledger manager. `hook` observes every manager model call
    /// and tool call. All knobs come from the config. The output root is
    /// the config's output dir. The worker shares the same tool state and
    /// runs under the private no-op hook.
    pub fn build(
        client: &rig::providers::ollama::Client,
        config: &crate::util::config::Config,
        hook: impl rig::agent::AgentHook + 'static,
    ) -> Agent {
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
        let worker = client
            .agent(&config.run.model)
            .name("ledger_worker")
            .preamble(WORKER_PREAMBLE)
            .tool(write)
            .tool(bash)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .additional_params(serde_json::json!({
                "num_ctx": config.run.num_ctx,
                "think": config.run.think,
            }))
            .add_hook(NoopHook)
            .build();
        // The manager. No direct tools: the worker tool is the only lever.
        client
            .agent(&config.run.model)
            .name("ledger_manager")
            .preamble(MANAGER_PREAMBLE)
            .dynamic_tool(worker.into_tool())
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

    /// Run the ledger manager over one task. `max_turns` bounds the
    /// model-call budget. Returns the structured result artifact. The
    /// model must deliver it through the output-tool call.
    pub async fn run(agent: &Agent, task: &MonolithRequest, max_turns: usize) -> anyhow::Result<LedgerResponse> {
        crate::util::task::task(agent, task, max_turns).await
    }
}

// Output artifact owned exclusively by this agent. Per the
// artifact-ownership rule, the DTO lives here. Per the Stepdown Rule, it
// sits below the entry functions as a secondary type that serves them. The
// input artifact is the shared [`MonolithRequest`].
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
