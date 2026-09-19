//! ## The ReCode agent
//!
//! The multi-agent pipeline of arXiv:2604.07341: ReCodeAgent translates a
//! whole repository across languages with four coordinated roles. The
//! analyzer inspects the source, the planning stage writes the translation
//! units, the translator synthesizes each unit, and the validator runs the
//! target toolchain. In the paper the roles talk through the Model Context
//! Protocol. Here the same phases live inside one manager loop: the
//! manager dispatches one pipeline phase per call to a fresh-context worker
//! agent through the worker tool, reads the worker report, and advances to
//! the next phase. The loop supports iterative repair: the manager sends
//! the validate phase again after a correction.
//!
//! The worker is a stripped monolith: one preamble with the working rules,
//! the `write` and `bash` tools, and no output schema. The manager carries
//! the typed [`RecodeResponse`] through `output_schema` with
//! `OutputMode::Tool`.

use crate::monolith::MonolithRequest;
use rig::agent::{Agent, OutputMode};
use rig::client::AgentClientExt;
use tools::{Bash, Write};

/// Preamble for the ReCode worker. Working rules only. The task data
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
- A coordinator assigns one pipeline phase per call: analyze the \
source, plan the translation units, translate the units, or validate \
the build and tests. Do the one phase in the message and report what \
it produced.";

/// Preamble for the ReCode manager. The loop: run the pipeline phases in
/// order, delegate one phase per call, validate, stop when done.
const MANAGER_PREAMBLE: &str = "\
You coordinate the translation of the codebase in the task. Run the \
pipeline in order: analyze the source, plan the translation units, \
translate every unit, validate the build and the tests. You have one \
worker tool. Give the worker the full instructions for one phase per \
call. The worker writes files and runs commands in the shared \
workspace. Track which phases are done from the worker reports. Stop \
and report the final result when all phases are done. Do not write \
files yourself.";

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

/// The ReCode agent namespace. `Recode::build` wires the manager and the
/// worker, `Recode::run` executes one task.
pub struct Recode;

impl Recode {
    /// Build the ReCode manager. `hook` observes every manager model call
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
        // The worker. Fresh context per delegated phase: the manager spawns
        // it through the worker tool on every call.
        let worker = client
            .agent(&config.run.model)
            .name("recode_worker")
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
            .name("recode_manager")
            .preamble(MANAGER_PREAMBLE)
            .dynamic_tool(worker.into_tool())
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .additional_params(serde_json::json!({
                "num_ctx": config.run.num_ctx,
                "think": config.run.think,
            }))
            .output_schema::<RecodeResponse>()
            .output_mode(OutputMode::Tool)
            .add_hook(hook)
            .build()
    }

    /// Run the ReCode manager over one task. `max_turns` bounds the
    /// model-call budget. Returns the structured result artifact. The
    /// model must deliver it through the output-tool call.
    pub async fn run(agent: &Agent, task: &MonolithRequest, max_turns: usize) -> anyhow::Result<RecodeResponse> {
        crate::util::task::task(agent, task, max_turns).await
    }
}

// Output artifact owned exclusively by this agent. Per the
// artifact-ownership rule, the DTO lives here. Per the Stepdown Rule, it
// sits below the entry functions as a secondary type that serves them. The
// input artifact is the shared [`MonolithRequest`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RecodeResponse {
    /// Build outcome of the translated codebase: `pass` when the build
    /// succeeds, `fail` otherwise.
    pub compilation_status: String,
    /// Fraction of translated tests that pass. `None` when the target has
    /// no test suite.
    pub test_pass_rate: Option<f64>,
    /// Number of repositories the pipeline translated. One per run.
    pub repos_translated: u32,
    /// Number of files the pipeline wrote.
    pub files_written: u32,
    /// One-line summary of the translation approach, for the validator's context.
    pub approach: String,
}
