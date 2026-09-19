//! Validator agent of the ReCode method. It runs the target toolchain
//! and reports failures and coverage gaps.

use crate::util::config::Config;
use rig::agent::{Agent, OutputMode};
use rig::client::AgentClientExt;
use tools::{Bash, Write};

/// Preamble for the validator agent. Working rules and the role duties
/// of the paper's validation phase. The translator report travels in
/// the prompt payload.
const PREAMBLE: &str = "\
You are the validator of a repository translation pipeline. You check \
the translated codebase in the workspace and report the result.\n\
\n\
Work rules:\n\
- Run the build and the test command from the task with the bash tool \
from the workspace root. Do not fix anything yourself.\n\
- Collect every failing test and its diagnostics into report.md in \
the workspace with the write tool.\n\
- Compare the tested functions against the plan's function list. \
Record every function without test coverage as a coverage gap.\n\
- Test generation duty: when the message asks for it, generate \
additional tests for uncovered functions. The tests must express the \
same behavior in both source and target semantics. Never weaken an \
existing test.\n\
- Work only inside your workspace. Do not modify translated source.\n\
- When you finish the validation, report the structured result.";

/// The validator agent namespace. [`Validator::build`] wires the agent.
pub struct Validator;

impl Validator {
    /// Build the validator agent. The agent runs the toolchain with the
    /// write and bash tools and returns a structured validation report.
    /// `hook` observes every model call and tool call.
    pub fn build(
        client: &rig::providers::ollama::Client,
        config: &Config,
        write: Write,
        bash: Bash,
        hook: impl rig::agent::AgentHook + 'static,
    ) -> Agent {
        client
            .agent(&config.run.model)
            .name("recode_validator")
            .preamble(PREAMBLE)
            .tool(write)
            .tool(bash)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .additional_params(serde_json::json!({
                "num_ctx": config.run.num_ctx,
                "think": config.run.think,
            }))
            .output_schema::<ValidationReport>()
            .output_mode(OutputMode::Tool)
            .add_hook(hook)
            .build()
    }
}

/// Structured validation report of the validator phase.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ValidationReport {
    /// True when the build succeeds and every test passes.
    pub all_success: bool,
    /// Number of tests that passed.
    pub tests_passed: u32,
    /// Number of tests that failed.
    pub tests_failed: u32,
    /// Functions from the plan without test coverage.
    pub uncovered_functions: Vec<String>,
    /// One entry per failing test with its diagnostics summary.
    pub failures: Vec<String>,
}
