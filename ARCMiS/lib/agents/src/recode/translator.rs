//! Translator agent of the ReCode method. It executes the plan and
//! fixes failures from the validation report.

use crate::util::config::Config;
use rig::agent::Agent;
use rig::client::AgentClientExt;
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
- When you finish the plan or the fixes, reply with a short plain \
text summary of what you did.";

/// The translator agent namespace. [`Translator::build`] wires the agent.
pub struct Translator;

impl Translator {
    /// Build the translator agent. The agent executes the plan with the
    /// write and bash tools. Its deliverable is the workspace plus a
    /// plain text ack, so it has no output schema. `hook` observes every
    /// model call and tool call.
    pub fn build(
        client: &rig::providers::ollama::Client,
        config: &Config,
        write: Write,
        bash: Bash,
        hook: impl rig::agent::AgentHook + 'static,
    ) -> Agent {
        client
            .agent(&config.run.model)
            .name("recode_translator")
            .preamble(PREAMBLE)
            .tool(write)
            .tool(bash)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .additional_params(serde_json::json!({
                "num_ctx": config.run.num_ctx,
                "think": config.run.think,
            }))
            .add_hook(hook)
            .build()
    }
}

/// Plain text ack of the translator phase. The real deliverable is the
/// workspace state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TranslatorReport {
    /// Short summary of what the translator did in this round.
    pub summary: String,
}
