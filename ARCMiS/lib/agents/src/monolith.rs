//! ## The monolith agent
//!
//! One agent that does the whole translation: it receives a
//! [`MonolithRequest`] (the whole input codebase plus the target toolchain),
//! works inside the output workspace with the `write` and `bash` tools, and
//! reports one [`MonolithResponse`]. The loop is rig's classic agent loop: the
//! model calls tools through the provider tool-call protocol, rig executes
//! them and feeds results back, and the output schema constrains the final answer to the
//! result schema through `output_schema` with `OutputMode::Tool`. Rig
//! registers the schema as a synthetic output tool. Verified in rig-agent
//! 0.42 `agent/run/mod.rs`: the output-tool call finalizes the run.
//!
//! The agent is thin: one preamble with the working rules, one typed input,
//! two tools, one typed output. No custom loop code.

use crate::util::config::Config;
use crate::util::provider::Provider;
use rig::agent::{Agent, AgentHook, OutputMode};
use rig::client::AgentClientExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tools::{Bash, Write};

/// Preamble for the monolith agent. Working rules only. The task data
/// travels in the typed prompt payload.
const PREAMBLE: &str = "\
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
further, report with the final report.";

/// The monolith agent namespace. `Monolith::build` wires the agent,
/// `Monolith::run` executes one task.
pub struct Monolith;

impl Monolith {
    /// Build the monolith agent. `hook` observes every model call and tool
    /// call. All knobs come from the config. The output root is the config's
    /// output dir. Generic over the provider client: ollama and the
    /// netmind gateway build the same agent shape.
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
        let mut builder = client
            .agent(&config.run.model)
            .name("monolith")
            .preamble(PREAMBLE)
            .tool(write)
            .tool(bash)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .output_schema::<MonolithResponse>()
            .output_mode(OutputMode::Tool)
            .add_hook(hook);
        if let Some(params) = provider.extra_params(&config.run) {
            builder = builder.additional_params(params);
        }
        builder.build()
    }

    /// Run the monolith agent over one task. `max_turns` bounds the
    /// model-call budget. `max_retries` bounds whole-task retries. Returns
    /// the structured result artifact. The model must deliver it through
    /// the output-tool call.
    pub async fn run(
        agent: &Agent,
        task: &MonolithRequest,
        max_turns: usize,
        max_retries: u32,
    ) -> anyhow::Result<MonolithResponse> {
        crate::util::task::task(agent, task, max_turns, max_retries).await
    }
}

// Input and output artifacts owned exclusively by this agent. Per the
// artifact-ownership rule, the DTOs live here. Per the Stepdown Rule, they
// sit below the entry functions as secondary types that serve them.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MonolithRequest {
    /// Input codebase: ordered map of relative path to file content.
    pub sources: BTreeMap<String, String>,
    /// Source language of the input codebase.
    pub source_language: String,
    /// Target language to translate into.
    pub target_language: String,
    /// Test command for the translated codebase, run in the output root.
    pub test_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MonolithResponse {
    /// Absolute or workspace-relative path of the output codebase root.
    pub output_dir: String,
    /// Number of files the agent wrote.
    pub files_written: u32,
    /// One-line summary of the translation approach, for the validator's context.
    pub approach: String,
}
