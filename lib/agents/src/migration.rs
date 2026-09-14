//! The migration agent. One agent, two tools: write the output codebase,
//! run the toolchain. Source contents are pre-seeded into the prompt (a 2B
//! model wanders when it must discover inputs itself), so the tool loop is
//! write + build + fix only. Task-specific values come from `Config`; see
//! .omp/rules/config.md.

use ::arcmis_tools::migration::{RunCommand, WriteFile};
use ::rig::prelude::*;
use ::rig::providers::ollama;

use crate::config::Config;

/// Preamble for the migration agent. States the loop contract; the task
/// specifics travel in the prompt from config.
const PREAMBLE: &str = "\
You translate a source codebase to the target language. The full input \
sources are already in the task message. The output workspace already \
holds a compilable package skeleton with stub functions: your job is to \
replace the stubs with the real translation and add the tests. \
Rules:\n\
- Do not touch the package manifest. Never write Cargo.toml or Cargo.lock.\n\
- One tool call per turn. No text-only turns.\n\
- All write_file paths are relative to the output workspace root (for \
example 'src/lib.rs'). Absolute paths are rejected.\n\
- Work only with write_file and run_command. Do not explore or list files.\n\
- Translate the listed test files exactly as-is: same cases, same \
expectations, adapted to the target language and its standard test runner.\n\
- Build with run_command. Fix every error with write_file until the build \
passes. Then run the tests. If tests fail, fix the code and repeat. Never \
weaken or delete a test to make it pass.\n\
- When everything passes, stop.";

/// Build the migration agent from a loaded config. Model, tool roots, turn
/// budget, and context size all come from the config.
pub fn build(config: &Config) -> ::rig::agent::Agent {
    // from_env reads OLLAMA_API_BASE_URL (defaults to localhost:11434) and
    // OLLAMA_API_KEY (optional). Verified in the published rig-core 0.42.
    let client = ollama::Client::from_env().expect("ollama client");
    let output_root = config.output.dir.clone();
    client
        .agent(&config.run.model)
        .preamble(PREAMBLE)
        .tool(WriteFile {
            root: ::std::clone::Clone::clone(&output_root),
            protected: ::arcmis_tools::migration::ProtectedFiles(
                ::std::clone::Clone::clone(&config.source.target.protected_files),
            ),
        })
        .tool(RunCommand {
            cwd: output_root,
        })
        .temperature(config.run.temperature)
        // Ollama context window: a 2B model defaults to 2048 tokens, which
        // truncates this task. Rig forwards extra params into ollama options.
        // Flat num_ctx: the published ollama adapter merges extra params into
        // ollama options directly (see its tests). Nested "options" is ignored.
        .additional_params(::serde_json::json!({
                "num_ctx": config.run.num_ctx,
                "think": config.run.think,
            }))
        .max_tokens(config.run.max_output_tokens)
        .build()
}

/// Read the configured source files and render them into the prompt.
/// File lists live in the config; contents come from disk at run time.
fn render_sources(config: &Config) -> ::core::result::Result<String, ::std::string::String> {
    let mut out = ::std::string::String::new();
    for section in [("SOURCE FILE", &config.run.source_files), ("TEST FILE (translate as-is)", &config.run.test_files)]
    {
        for rel in section.1 {
            let path = config.source.root.join(rel);
            let content = ::std::fs::read_to_string(&path)
                .map_err(|e| format!("source read failed for {}: {e}", path.display()))?;
            out.push_str(&format!("=== {} {} ===\n{content}\n", section.0, rel));
        }
    }
    Ok(out)
}

/// The user prompt, assembled from config values plus the seeded sources.
pub fn prompt(config: &Config) -> ::core::result::Result<String, ::std::string::String> {
    let sources = render_sources(config)?;
    let hint = &config.source.target.style_hint;
    Ok(format!(
        "Translate this {} codebase to {}. \
Write the output as a {} package into the output workspace. \
Build it and run the tests with `{}`. Then report the final build and test status.\n\
Style: {hint}\n\n{sources}",
        config.source.language,
        config.source.target.language,
        config.source.target.language,
        config.source.target.test_command,
        hint = hint,
        sources = sources,
    ))
}

/// Run the agent against the config. Returns the model's final text.
pub async fn run(
    config: &Config,
    hook: impl ::rig::agent::AgentHook + 'static,
) -> ::core::result::Result<String, ::rig::completion::PromptError> {
    let agent = build(config);
    let task = prompt(config).map_err(|e| {
        ::rig::completion::PromptError::CompletionError(::rig::completion::CompletionError::RequestError(
            ::std::boxed::Box::new(::std::io::Error::other(e)),
        ))
    })?;
    agent.prompt(task).max_turns(config.run.max_turns).add_hook(hook).await
}
