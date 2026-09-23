//! Translator agent of the ReCode method. It executes the plan and
//! fixes failures from the validation report.

use crate::util::config::Config;
use crate::util::provider::Provider;
use rig::agent::{Agent, AgentHook, OutputMode};
use rig::client::AgentClientExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tools::{Bash, Write};

/// Preamble for the translator agent. Working rules and the role duties
/// of the paper's translation and fix phases. The plan data travels
/// in the prompt payload.
const PREAMBLE: &str = "\
You are the translator of a repository translation pipeline. You \
execute the implementation plan in the task: fill the skeleton files \
in the workspace with real code, Part A first, then Part B.\n\
\n\
Work rules:\n\
- Translate each plan unit in order. Preserve the name mapping from \
the plan. Write every file with the write tool.\n\
- Translate tests faithfully. Do not weaken or delete an assertion to \
make it pass.\n\
- Build with the bash tool after each part. Fix every build error \
before you continue.\n\
- Work only inside your workspace.\n\
- Fix mode: when the message carries a validation report, fix the \
reported failures and nothing else. Rebuild and rerun the failing \
tests after each fix.\n\
- When you finish the plan or the fixes, reply with a short JSON \
object with one key, summary, whose value is a short summary of \
what you did.";

/// The translator agent namespace. [`Translator::build`] wires the agent.
pub struct Translator;
impl Translator {
    /// Build the translator agent. The agent executes the plan with the
    /// write and bash tools and closes the phase with a structured ack.
    /// `hook` observes every model call and tool call.
    pub fn build<C>(
        client: &C,
        config: &Config,
        provider: &Provider,
        write: Write,
        bash: Bash,
        hook: impl AgentHook + 'static,
    ) -> Agent
    where
        C: AgentClientExt,
        C::CompletionModel: 'static,
    {
        let mut builder = client
            .agent(&config.run.model)
            .name("recode_translator")
            .preamble(PREAMBLE)
            .tool(write)
            .tool(bash)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .output_schema::<TranslatorReport>()
            .output_mode(OutputMode::Tool)
            .add_hook(hook);
        if let Some(params) = provider.extra_params(&config.run) {
            builder = builder.additional_params(params);
        }
        builder.build()
    }
}

/// Structured ack of the translator phase. The real deliverable is the
/// workspace state. The schema rides the prompt (OutputMode::Prompted)
/// and the helper parses the final text, so the weak local model needs
/// no tool call to close the phase.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TranslatorReport {
    /// Short summary of what the translator did in this round.
    pub summary: String,
}
