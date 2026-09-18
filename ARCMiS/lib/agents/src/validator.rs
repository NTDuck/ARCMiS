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

use ::rig::agent::{Agent, OutputMode};
use ::rig::client::AgentClientExt;
use ::rig::completion::Prompt;
use ::tools::Bash;

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

/// Build the validator agent. The bash tool roots at the codebase
/// under validation. The model cannot escape that root through the tool.
/// `hook` observes every model call and tool call. All knobs come from the
/// config.
pub fn build(
    client: &::rig::providers::ollama::Client,
    config: &crate::util::config::Config,
    hook: impl ::rig::agent::AgentHook + 'static,
) -> Agent {
    let bash = Bash {
        root: config.output.dir.clone(),
    };
    client
        .agent(&config.run.model)
        .name("validator")
        .preamble(PREAMBLE)
        .tool(bash)
        .temperature(config.run.temperature)
        .max_tokens(config.run.max_output_tokens)
        .additional_params(::serde_json::json!({
            "num_ctx": config.run.num_ctx,
            "think": config.run.think,
        }))
        .output_schema::<ValidatorResponse>()
        .output_mode(OutputMode::Tool)
        .add_hook(hook)
        .build()
}

/// Run the validator agent over one task. `max_turns` bounds the
/// model-call budget. Returns the structured result artifact. The model
/// must deliver it through the output-tool call.
pub async fn run(
    agent: &Agent,
    task: &ValidatorRequest,
    max_turns: usize,
) -> ::core::result::Result<ValidatorResponse, ::rig::completion::PromptError> {
    let prompt =
        ::serde_json::to_string(task).map_err(|error| request_error(::std::format!("task render failed: {error}")))?;
    let raw = Prompt::prompt(agent, prompt).max_turns(max_turns).await?;
    ::serde_json::from_str(&raw)
        .map_err(|error| request_error(::std::format!("validator result parse failed: {error}")))
}

// Input and output artifacts owned exclusively by this agent. Per the
// artifact-ownership rule, the DTOs live here. Per the block-order rule,
// they come after the agent entry functions as secondary types.
#[derive(::core::fmt::Debug, ::core::clone::Clone, ::serde::Serialize, ::serde::Deserialize, ::schemars::JsonSchema)]
pub struct ValidatorRequest {
    /// Path of the codebase root to validate.
    pub output_dir: ::std::string::String,
    /// Toolchain steps to run, in order. Example: `build`, `test`.
    pub toolchain: ::std::vec::Vec<::std::string::String>,
    /// Test command from the run config, for reference in the report.
    pub test_command: ::std::string::String,
    /// The monolith's own summary of its approach, for context only.
    pub approach: ::std::string::String,
}

#[derive(::core::fmt::Debug, ::core::clone::Clone, ::serde::Serialize, ::serde::Deserialize, ::schemars::JsonSchema)]
pub struct ValidatorStepOutcome {
    /// Step name, for example `build` or `test`.
    pub step: ::std::string::String,
    /// True when the step command exited successfully.
    pub passed: bool,
    /// Runner-reported pass counts, when the step ran tests.
    pub tests_passed: ::core::option::Option<u32>,
    /// Runner-reported failure counts, when the step ran tests.
    pub tests_failed: ::core::option::Option<u32>,
    /// Last lines of combined command output, for the report.
    pub detail_tail: ::std::string::String,
}

#[derive(::core::fmt::Debug, ::core::clone::Clone, ::serde::Serialize, ::serde::Deserialize, ::schemars::JsonSchema)]
pub struct ValidatorResponse {
    /// True when every toolchain step passed.
    pub compilation_status: ::std::string::String,
    /// Test pass rate as a fraction of reported tests, `None` when no test
    /// step reported counts.
    pub test_pass_rate: ::core::option::Option<f64>,
    /// Outcomes of the executed toolchain steps, in execution order.
    pub steps: ::std::vec::Vec<ValidatorStepOutcome>,
}

/// Wrap an internal failure into a rig prompt error.
fn request_error(message: ::std::string::String) -> ::rig::completion::PromptError {
    ::rig::completion::PromptError::CompletionError(::rig::completion::CompletionError::ProviderError(message))
}
