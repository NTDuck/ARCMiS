//! Planner agent of the ReCode method. It turns the design into a
//! dependency ordered implementation plan.

use crate::util::config::Config;
use crate::util::noop_hook::NoopHook;
use rig::agent::{Agent, OutputMode};
use rig::client::AgentClientExt;
use rig::providers::ollama::Client;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tools::{Bash, Write};

/// Preamble for the planner agent. Working rules and the role duties of
/// the paper's planning phase. The task and design data travel in the
/// typed prompt payload.
const PREAMBLE: &str = "\
You are the planner of a repository translation pipeline. You turn the \
design in the task into a dependency ordered implementation plan. Your \
work has four phases:\n\
\n\
1. Fragment extraction: list every function and class of the source \
project from the task and the design documents in the workspace.\n\
2. Name mapping: preserve the symbol names in the target language. \
Record every name that must change and why.\n\
3. Skeleton generation: write compilable skeleton files for the target \
project into the workspace with the write tool. Every function and \
class exists as a stub.\n\
4. Implementation plan: write plan.md in the workspace. Split the plan \
into Part A, the source files in bottom-up dependency order, and Part \
B, the test files in the same order.\n\
\n\
Work rules:\n\
- Write plan.md and the skeleton files with the write tool so later \
agents can read and fill them.\n\
- Keep the dependency order: a unit appears only after the units it \
depends on.\n\
- Work only inside your workspace.\n\
- When you finish the plan, report the structured plan.";

/// The planner agent namespace. [`Planner::build`] wires the agent.
pub struct Planner;

impl Planner {
    /// Build the planner agent. The agent writes the plan and the
    /// skeleton files with the write and bash tools and returns a
    /// structured plan. It runs under the no-op hook.
    pub fn build(client: &Client, config: &Config, write: Write, bash: Bash) -> Agent {
        client
            .agent(&config.run.model)
            .name("recode_planner")
            .preamble(PREAMBLE)
            .tool(write)
            .tool(bash)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .additional_params(serde_json::json!({
                "num_ctx": config.run.num_ctx,
                "think": config.run.think,
            }))
            .output_schema::<PlanningOutput>()
            .output_mode(OutputMode::Tool)
            .add_hook(NoopHook)
            .build()
    }
}

/// Structured implementation plan of the planner phase.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlanningOutput {
    /// Part A of the plan: source files to translate, in bottom-up
    /// dependency order.
    pub parts_a: Vec<String>,
    /// Part B of the plan: test files to translate, in bottom-up
    /// dependency order.
    pub parts_b: Vec<String>,
    /// Notes on the symbol name mapping between source and target.
    pub name_mapping_notes: String,
}
