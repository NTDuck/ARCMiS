//! The ledger worker. A stripped monolith that executes one delegated task
//! in the shared workspace. No output schema: the worker reports through
//! its tool result and through `notes.md`.

use crate::util::config::Config;
use crate::util::noop_hook::NoopHook;
use crate::util::provider::Provider;
use rig::agent::Agent;
use rig::client::AgentClientExt;
use rig::completion::Prompt;
use tools::{Bash, Write};

/// Preamble for the ledger worker. Working rules only. The task data
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
- A manager coordinates you. Do the single task in the message. Append \
what you did to notes.md in the workspace root.";

/// The ledger worker agent namespace. `Worker::build` wires the worker.
pub struct Worker;

impl Worker {
    /// Build the ledger worker. `write` and `bash` share the tool state
    /// that the parent build created, rooted at the config's output dir.
    /// The worker runs under the shared no-op hook: the outer hook observes
    /// the manager.
    pub fn build<C>(client: &C, config: &Config, provider: &Provider, write: Write, bash: Bash) -> Agent
    where
        C: AgentClientExt,
        C::CompletionModel: 'static,
    {
        let mut builder = client
            .agent(&config.run.model)
            .name("ledger_worker")
            .preamble(WORKER_PREAMBLE)
            .tool(write)
            .tool(bash)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .add_hook(NoopHook);
        if let Some(params) = provider.extra_params(&config.run) {
            builder = builder.additional_params(params);
        }
        builder.build()
    }
}

impl Worker {
    /// Prompt the worker over one delegated task with its own fresh turn
    /// budget. The paper's fresh-context worker starts every delegation
    /// with a full budget, not the manager's remaining one.
    pub async fn run(agent: &Agent, task: &str, max_turns: usize, max_retries: u32) -> anyhow::Result<String> {
        let prompt = format!("You are the ledger worker. Task:\n{task}");
        let mut last_error = None;
        for attempt in 0..=max_retries {
            if attempt > 0 {
                tracing::warn!(attempt, "worker retry after failed attempt");
            }
            match agent.prompt(&prompt).max_turns(max_turns).await {
                Ok(raw) => return Ok(raw),
                Err(error) => last_error = Some(error),
            }
        }
        Err(anyhow::anyhow!(last_error.expect("retry loop ran at least once")).context("worker failed after retries"))
    }
}
