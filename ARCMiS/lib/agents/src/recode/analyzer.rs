//! Analyzer agent of the ReCode method. It researches the source project
//! and designs the target project.

use crate::util::config::Config;
use crate::util::noop_hook::NoopHook;
use crate::util::provider::Provider;
use rig::agent::{Agent, OutputMode};
use rig::client::AgentClientExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tools::{Bash, Write};

/// Preamble for the analyzer agent. Working rules and the role duties of
/// the paper's research and design phases. The task data travels in the
/// typed prompt payload.
const PREAMBLE: &str = "\
You are the analyzer of a repository translation pipeline. You research \
the source codebase given in the task and design its translation into \
the target language. Your work has three phases:\n\
\n\
1. Source project research: read the source files with the bash tool \
(cat, ls, grep) and the write tool. Understand the module structure, \
the public interfaces, and the build and test setup.\n\
2. Third-party library analysis: for every source dependency, find the \
idiomatic counterpart library in the target language and note version \
and API differences.\n\
3. Target project design: write your research and design into \
design.md in the workspace. Cover the target module structure, the \
dependency mapping, and the risks of the translation.\n\
\n\
Work rules:\n\
- Write design.md with the write tool so later agents can read it.\n\
- Work only inside your workspace. Do not modify the source project.\n\
- When you finish the design document, report the structured summary.";

/// The analyzer agent namespace. [`Analyzer::build`] wires the agent.
pub struct Analyzer;

impl Analyzer {
    /// Build the analyzer agent. The agent explores the source with the
    /// write and bash tools and returns a structured research and design
    /// report. It runs under the no-op hook.
    pub fn build<C>(client: &C, config: &Config, provider: &Provider, write: Write, bash: Bash) -> Agent
    where
        C: AgentClientExt,
        C::CompletionModel: 'static,
    {
        let mut builder = client
            .agent(&config.run.model)
            .name("recode_analyzer")
            .preamble(PREAMBLE)
            .tool(write)
            .tool(bash)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .output_schema::<AnalyzerReport>()
            .output_mode(OutputMode::Tool)
            .add_hook(NoopHook);
        if let Some(params) = provider.extra_params(&config.run) {
            builder = builder.additional_params(params);
        }
        builder.build()
    }
}

/// Structured research and design report of the analyzer phase.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzerReport {
    /// One-paragraph summary of the target project design. The full
    /// design lives in design.md in the workspace.
    pub design_summary: String,
    /// Third-party libraries the source project depends on.
    pub dependencies: Vec<String>,
    /// Idiomatic target-language counterpart for each dependency.
    pub target_libraries: Vec<String>,
}
