//! The ledger worker. A stripped monolith that executes one delegated task
//! in the shared workspace. No output schema: the worker reports through
//! its tool result and through `notes.md`.

use crate::util::config::Config;
use crate::util::noop_hook::NoopHook;
use rig::agent::Agent;
use rig::client::AgentClientExt;
use rig::providers::ollama::Client;
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
    pub fn build(client: &Client, config: &Config, write: Write, bash: Bash) -> Agent {
        client
            .agent(&config.run.model)
            .name("ledger_worker")
            .preamble(WORKER_PREAMBLE)
            .tool(write)
            .tool(bash)
            .temperature(config.run.temperature)
            .max_tokens(config.run.max_output_tokens)
            .additional_params(serde_json::json!({
                "num_ctx": config.run.num_ctx,
                "think": config.run.think,
            }))
            .add_hook(NoopHook)
            .build()
    }
}
