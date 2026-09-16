//! ## The default agent
//!
//! The default agent translates a source codebase to a target language.
//! One agent, two tools: write the output codebase, run the toolchain.
//! The harness walks the input root and passes every readable file as
//! content in the task message, so the tool loop is write, build, test,
//! fix. The agent plans and writes the whole output package itself: it
//! generates the manifest and the directory structure; the code supplies
//! no paths and no file contents. Task-specific values come from `Config`;
//! see .omp/rules/config.md.
//!
//! Steps:
//! 1. `build` creates the rig agent with the Ollama client, the preamble,
//!    the write and run tools, and the model parameters from config.
//! 2. `prompt` renders the collected `Sources` plus the languages and the
//!    test command into the task message.
//! 3. `run` drives the agent loop for the configured turn budget and
//!    returns the model's final text.

use ::tools::{RunCommand, WriteFile};
use ::rig::prelude::*;

use crate::config::Config;
use crate::sources;

/// Preamble for the default agent. States the loop contract. The task
/// specifics travel in the prompt from config and the collected sources.
const PREAMBLE: &str = "\
You translate a source codebase to a target language. The full input \
sources are already in the task message. You own the whole output \
package: plan its structure, generate the manifest, and create every \
file and directory yourself. Rules:\n\
- One tool call per turn. No text-only turns.\n\
- All write_file paths are relative to the output root. Absolute paths \
are rejected.\n\
- Work only with write_file and run_command. Do not explore or list files.\n\
- Do not weaken or delete a translated test to make it pass.\n\
- Build with run_command. Fix every error with write_file until the \
build passes. Then run the tests. If tests fail, fix the code and \
repeat.\n\
- When everything passes, stop.";

/// Build the default agent from a loaded config. Model, tool roots, turn
/// budget, and context size all come from the config.
pub fn build(config: &Config) -> ::rig::agent::Agent {
    // from_env reads OLLAMA_API_BASE_URL (defaults to localhost:11434) and
    // OLLAMA_API_KEY (optional). Verified in the published rig-core 0.42.
    let client = ::rig::providers::ollama::Client::from_env().expect("ollama client");
    let output_root = ::std::clone::Clone::clone(&config.output.dir);
    client
        .agent(&config.run.model)
        .preamble(PREAMBLE)
        .tool(WriteFile {
            root: ::std::clone::Clone::clone(&output_root),
        })
        .tool(RunCommand {
            cwd: output_root,
        })
        .temperature(config.run.temperature)
        // Ollama context window: a small model defaults to a narrow window,
        // which truncates this task. Rig forwards extra params into ollama
        // options. Flat num_ctx: the published ollama adapter merges extra
        // params into ollama options directly (see its tests). Nested
        // "options" is ignored.
        .additional_params(::serde_json::json!({
                "num_ctx": config.run.num_ctx,
                "think": config.run.think,
            }))
        .max_tokens(config.run.max_output_tokens)
        .build()
}

/// The user prompt, assembled from config values plus the collected
/// sources. The prompt names no files and no paths; the agent plans the
/// output structure itself.
pub fn prompt(config: &Config, sources: &sources::Sources) -> ::core::result::Result<String, ::std::string::String> {
    if sources.is_empty() {
        return ::core::result::Result::Err(format!(
            "no readable sources collected from {}",
            config.source.root.display()
        ));
    }
    Ok(format!(
        "Translate this {} codebase to {}. \
Write the output as a complete {} package into the output workspace: \
plan the package structure, generate the manifest, and write every \
file with write_file. \
Build it and run the tests with `{}`. Then report the final build and test status.\n\n{}",
        config.source.language,
        config.source.target.language,
        config.source.target.language,
        config.source.target.test_command,
        sources::render(sources),
    ))
}

/// Run the agent against the config. Collects the sources from the input
/// root and returns the model's final text.
pub async fn run(
    config: &Config,
    hook: impl ::rig::agent::AgentHook + 'static,
) -> ::core::result::Result<String, ::rig::completion::PromptError> {
    let per_file_cap = config.run.num_ctx / 4 * sources::BYTES_PER_TOKEN;
    let collected = sources::collect(&config.source.root, per_file_cap)
        .map_err(request_error)
        .inspect_err(|error| ::tracing::warn!(error = %error, "source collection failed"))?;
    let agent = build(config);
    let task = prompt(config, &collected).map_err(request_error)?;
    agent.prompt(task).max_turns(config.run.max_turns).add_hook(hook).await
}

/// Wrap an internal failure into a rig prompt error. The agent loop
/// surfaces plain text errors as request errors.
fn request_error(error: ::std::string::String) -> ::rig::completion::PromptError {
    ::rig::completion::PromptError::CompletionError(::rig::completion::CompletionError::RequestError(
        ::std::boxed::Box::new(::std::io::Error::other(error)),
    ))
}
