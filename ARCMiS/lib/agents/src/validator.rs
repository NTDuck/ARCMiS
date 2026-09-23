//! ## The validator agent
//!
//! One agent that validates an output codebase: it receives a
//! [`ValidatorRequest`] (the codebase root, the toolchain steps to run, the
//! test command). It executes each step through the `bash` tool and reports
//! one [`ValidatorResponse`]. The result schema forces depth: one
//! [`ValidatorStepOutcome`] per step with pass flags, test counts, and output tails.
//!
//! The agent is thin: one preamble, one typed input, one tool, one typed
//! output. No custom loop code.

use crate::util::config::Config;
use crate::util::provider::Provider;
use rig::agent::{Agent, AgentHook, OutputMode};
use rig::client::AgentClientExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tools::Bash;

/// Preamble for the validator agent. Working rules only. The task data
/// travels in the typed prompt payload.
const PREAMBLE: &str = "\
You validate the codebase at the directory given in the task. Run every \
toolchain step listed in the task, in order, with the bash tool, from \
the codebase root. For a test step, read the runner output and count \
passed and failed tests.\n\
\n\
Work rules:\n\
- Run each step once. Do not fix anything. Do not modify files.\n\
- Record the real outcome of every step. Never guess a pass.\n\
- When every step has run, report with the final report.";

/// The validator agent namespace. `Validator::build` wires the agent,
/// `Validator::run` executes one task.
pub struct Validator;

impl Validator {
    /// Build the validator agent. The bash tool roots at the codebase
    /// under validation. The model cannot escape that root through the tool.
    /// `hook` observes every model call and tool call. All knobs come from the
    /// config.
    pub fn build<C>(client: &C, config: &Config, provider: &Provider, hook: impl AgentHook + 'static) -> Agent
    where
        C: AgentClientExt,
        C::CompletionModel: 'static,
    {
        let bash = Bash {
            root: config.output.dir.clone(),
        };
        let mut builder = client
            .agent(&config.run.model)
            .name("validator")
            .preamble(PREAMBLE)
            .tool(bash)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .output_schema::<ValidatorResponse>()
            .output_mode(OutputMode::Tool)
            .add_hook(hook);
        if let Some(params) = provider.extra_params(&config.run) {
            builder = builder.additional_params(params);
        }
        builder.build()
    }

    /// Run the validator agent over one task. `max_turns` bounds the
    /// model-call budget. `max_retries` bounds whole-task retries. Returns
    /// the structured result artifact. The model must deliver it through
    /// the output-tool call.
    pub async fn run(
        agent: &Agent,
        task: &ValidatorRequest,
        max_turns: usize,
        max_retries: u32,
    ) -> anyhow::Result<ValidatorResponse> {
        crate::util::task::task(agent, task, max_turns, max_retries).await
    }
}

// Input and output artifacts owned exclusively by this agent. Per the
// artifact-ownership rule, the DTOs live here. Per the Stepdown Rule, they
// sit below the entry functions as secondary types that serve them.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidatorRequest {
    /// Path of the codebase root to validate.
    pub output_dir: String,
    /// Toolchain steps to run, in order. Example: `build`, `test`.
    pub toolchain: Vec<String>,
    /// Test command from the run config, for reference in the report.
    pub test_command: String,
    /// The monolith's own summary of its approach, for context only.
    pub approach: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidatorStepOutcome {
    /// Step name, for example `build` or `test`.
    pub step: String,
    /// True when the step command exited successfully.
    pub passed: bool,
    /// Runner-reported pass counts, when the step ran tests.
    pub tests_passed: Option<u32>,
    /// Runner-reported failure counts, when the step ran tests.
    pub tests_failed: Option<u32>,
    /// Last lines of combined command output, for the report.
    pub detail_tail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidatorResponse {
    /// True when every toolchain step passed.
    pub compiled: bool,
    /// Test pass rate as a fraction of reported tests, `None` when no test
    /// step reported counts.
    pub test_pass_rate: Option<f64>,
    /// Outcomes of the executed toolchain steps, in execution order.
    pub steps: Vec<ValidatorStepOutcome>,
}
