//! Reporter agent of the ReCode method. It turns the final validation
//! report into the typed response.

use crate::recode::RecodeResponse;
use crate::util::config::Config;
use crate::util::noop_hook::NoopHook;
use crate::util::provider::Provider;
use rig::agent::{Agent, OutputMode};
use rig::client::AgentClientExt;

/// Preamble for the reporter agent. Working rules and the role duty of
/// the paper's final report step. The validation report travels in the
/// prompt payload.
const PREAMBLE: &str = "\
You are the reporter of a repository translation pipeline. You read \
the validation report in the message and the workspace state, and you \
emit the final structured response. Do not modify any file. Report \
the compilation status and the test pass rate. Report the number of \
translated repositories and the number of files written. Report a \
one-line summary of the translation approach.";

/// The reporter agent namespace. [`Reporter::build`] wires the agent.
pub struct Reporter;

impl Reporter {
    /// Build the reporter agent. The agent has no tools: it reads the
    /// validation report in the message and emits the typed response.
    /// It runs under the no-op hook.
    pub fn build<C>(client: &C, config: &Config, provider: &Provider) -> Agent
    where
        C: AgentClientExt,
        C::CompletionModel: 'static,
    {
        let mut builder = client
            .agent(&config.run.model)
            .name("recode_reporter")
            .preamble(PREAMBLE)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .output_schema::<RecodeResponse>()
            .output_mode(OutputMode::Tool)
            .add_hook(NoopHook);
        if let Some(params) = provider.extra_params(&config.run) {
            builder = builder.additional_params(params);
        }
        builder.build()
    }
}
