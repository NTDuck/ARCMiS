//! ## The default agent — ReAct
//!
//! One agent that translates a source codebase to a target language. The
//! loop follows ReAct (Yao et al., arXiv:2210.03629) exactly: at every step
//! the model emits one Thought and one Action as text, the harness executes
//! the action and appends the Observation, and the next step sees the full
//! trajectory. A `Finish[answer]` action ends the episode. Nothing outside
//! the paper is added: no planning trees, no reflection, no self-consistency,
//! no memory beyond the raw trajectory. Thought and Action travel in one
//! assistant message (the paper's exemplars show `Thought N` then `Action N`
//! as consecutive lines); the Observation is injected as a user message.
//!
//! Steps:
//! 1. `build` creates the completion model with the ReAct preamble and the
//!    model parameters from config.
//! 2. `prompt` renders the collected `Sources` plus the languages and the
//!    test command into the task message.
//! 3. `run` drives the Thought/Action/Observation loop for the configured
//!    turn budget and returns the answer of the `Finish` action.
//!
//! The two domain actions are bracketed like the paper's `Search[x]`:
//! `write` carries one JSON object for the write tool, `bash` carries one
//! shell line. Both execute through the tools crate.

use ::rig::completion::{AssistantContent, CompletionModel, Message};
use ::rig::prelude::*;

use ::tools::{Bash, Write};

use crate::util::config::Config;
use crate::util::sources;

/// Preamble for the ReAct agent. States the task contract. The action
/// vocabulary travels here; the task specifics travel in the prompt.
const PREAMBLE: &str = "\
You translate a source codebase to a target language. The full input \
sources are already in the task message. You own the whole output \
package: plan its structure, generate the manifest, and create every \
file and directory yourself.\n\
\n\
Solve the task with alternating Thought and Action lines:\n\
Thought: <your reasoning about the step>\n\
Action: <one action>\n\
Then the harness answers with an Observation. Never emit an \
Observation line yourself. Actions:\n\
- Write[{\"path\": \"...\", \"content\": \"...\"}] - write one file into \
the output workspace. Paths are relative to the output root.\n\
- Bash[<shell line>] - run one shell line in the output workspace.\n\
- Finish[<answer>] - end the task with the final build and test status.\n\
Rules:\n\
- One Action per Thought. Wait for the Observation before the next \
Thought.\n\
- Work only with Write and Bash. Do not explore or list files.\n\
- Do not weaken or delete a translated test to make it pass.\n\
- Build with Bash. Fix every error with Write until the build passes. \
Then run the tests. If tests fail, fix the code and repeat.\n\
- When everything passes, end with Finish.";

/// The ReAct step budget. The paper uses 7 steps for its reasoning tasks
/// and reports that more steps do not help. The configured max_turns caps
/// the model-call count, so this budget bounds the trajectory.
const REACT_MAX_STEPS: usize = 12;

/// One action the model emitted: bracketed text like `Write[...]`.
#[derive(::core::fmt::Debug)]
pub struct EmittedAction {
    /// The action name: `Write`, `Bash`, or `Finish`.
    pub name: String,
    /// The bracketed argument text.
    pub argument: String,
}

/// Build the completion model for the default agent. Model parameters all
/// come from the config. Temperature and the ollama window params apply per
/// request in `model_turn`; the raw completion model carries none of them.
pub fn build(config: &Config) -> ::rig::providers::ollama::CompletionModel {
    // from_env reads OLLAMA_API_BASE_URL (defaults to localhost:11434) and
    // OLLAMA_API_KEY (optional). Verified in the published rig-core 0.42.
    let client = ::rig::providers::ollama::Client::from_env().expect("ollama client");
    client.completion_model(&config.run.model)
}

/// The user prompt, assembled from config values plus the collected
/// sources. The prompt names no files and no paths; the agent plans the
/// output structure itself.
pub fn prompt(config: &Config, sources: &sources::Sources) -> ::core::result::Result<String, ::std::string::String> {
    if sources.is_empty() {
        return ::core::result::Result::Err(::std::format!(
            "no readable sources collected from {}",
            config.source.root.display()
        ));
    }
    Ok(::std::format!(
        "Translate this {} codebase to {}. \
Write the output as a complete {} package into the output workspace: \
plan the package structure, generate the manifest, and write every \
file with Write. \
Build it and run the tests with `{}`. Then report the final build and test status.\n\n{}",
        config.source.language,
        config.source.target.language,
        config.source.target.language,
        config.source.target.test_command,
        sources::render(sources),
    ))
}

/// Run the ReAct agent against the config. Collects the sources from the
/// input root, then drives the Thought/Action/Observation loop. Returns
/// the answer carried by the `Finish` action.
pub async fn run(
    config: &Config,
    hook: impl ::rig::agent::AgentHook + 'static,
) -> ::core::result::Result<String, ::rig::completion::PromptError> {
    let per_file_cap = config.run.num_ctx / 4 * sources::BYTES_PER_TOKEN;
    let collected = sources::collect(&config.source.root, per_file_cap)
        .map_err(request_error)
        .inspect_err(|error| ::tracing::warn!(error = %error, "source collection failed"))?;
    let task = prompt(config, &collected).map_err(request_error)?;
    react_loop(config, task, hook).await
}

/// Drive the ReAct Thought/Action/Observation loop.
async fn react_loop(
    config: &Config,
    task: String,
    hook: impl ::rig::agent::AgentHook + 'static,
) -> ::core::result::Result<String, ::rig::completion::PromptError> {
    let model = build(config);
    let output_root = ::std::clone::Clone::clone(&config.output.dir);
    let write_tool = Write {
        root: ::std::clone::Clone::clone(&output_root),
        snapshots: ::tools::SnapshotStore::new(),
    };
    let bash_tool = Bash {
        root: output_root,
    };
    let mut history: ::std::vec::Vec<Message> = ::std::vec::Vec::new();
    history.push(Message::user(task));
    for step in 1..=REACT_MAX_STEPS {
        let text = model_turn(config, &model, &history, step, &hook).await?;
        let action = match parse_action(&text) {
            ::core::option::Option::Some(action) => action,
            ::core::option::Option::None => {
                history.push(Message::assistant(text));
                history.push(Message::user(observation_text(
                    "Your reply had no Action line. Reply again with Thought: and Action:.",
                )));
                continue;
            },
        };
        history.push(Message::assistant(text));
        if action.name == "Finish" {
            return ::core::result::Result::Ok(action.argument);
        }
        let observation = execute_action(&action, &write_tool, &bash_tool).await;
        history.push(Message::user(observation_text(&observation)));
    }
    ::core::result::Result::Err(request_error(::std::format!(
        "ReAct budget of {REACT_MAX_STEPS} steps ran out before Finish"
    )))
}

/// Make one model call for one ReAct step.
async fn model_turn(
    config: &Config,
    model: &::rig::providers::ollama::CompletionModel,
    history: &[Message],
    step: usize,
    hook: &impl ::rig::agent::AgentHook,
) -> ::core::result::Result<String, ::rig::completion::PromptError> {
    ::tracing::info!(step, max_turns = config.run.max_turns, "react step");
    let request = model
        .completion_request("")
        .preamble(PREAMBLE.to_string())
        .messages(history.to_vec())
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
        .max_tokens(config.run.max_output_tokens);
    let response = request.send().await.map_err(|error| {
        ::rig::completion::PromptError::CompletionError(::rig::completion::CompletionError::ProviderError(
            error.to_string(),
        ))
    })?;
    let text = response
        .choice
        .iter()
        .filter_map(|part| match part {
            AssistantContent::Text(part) => ::core::option::Option::Some(part.text.clone()),
            _ => ::core::option::Option::None,
        })
        .collect::<::std::string::String>();
    let _ = hook;
    Ok(text)
}

/// Parse one `Name[argument]` action out of the model text. The last
/// bracketed action wins, matching how the exemplars close with Finish.
fn parse_action(text: &str) -> ::core::option::Option<EmittedAction> {
    let mut found: ::core::option::Option<EmittedAction> = ::core::option::Option::None;
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let open = match text[index..].find('[') {
            ::core::option::Option::Some(offset) => index + offset,
            ::core::option::Option::None => break,
        };
        let close = match text[open..].find(']') {
            ::core::option::Option::Some(offset) => open + offset,
            ::core::option::Option::None => break,
        };
        let head = text[..open].trim_end();
        let name =
            head.rsplit(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap_or("").trim_start().to_owned();
        if !name.is_empty() {
            found = ::core::option::Option::Some(EmittedAction {
                name,
                argument: text[open + 1..close].to_owned(),
            });
        }
        index = close + 1;
    }
    found
}

/// Render one observation as a user message. The paper injects the
/// environment reply verbatim under an `Observation` label.
fn observation_text(observation: &str) -> String {
    ::std::format!("Observation: {observation}")
}

/// Execute one parsed action through the matching tool. Unknown action
/// names surface as an observation, never as a crash.
async fn execute_action(action: &EmittedAction, write_tool: &Write, bash_tool: &Bash) -> String {
    match action.name.as_str() {
        "Write" => run_tool(write_tool, &action.argument).await,
        "Bash" => run_tool(bash_tool, &action.argument).await,
        other => ::std::format!("Unknown action '{other}'. Use Write[...], Bash[...], or Finish[...]."),
    }
}

/// Call one tool with a JSON argument payload and render its output.
async fn run_tool<T>(tool: &T, argument: &str) -> String
where
    T: ::rig::tool::Tool,
    T::Args: ::serde::de::DeserializeOwned,
{
    let args: T::Args = match ::serde_json::from_str(argument) {
        ::core::result::Result::Ok(args) => args,
        ::core::result::Result::Err(error) => {
            return ::std::format!("bad arguments for {name}: {error}", name = T::NAME);
        },
    };
    let context = &mut ::rig::tool::ToolContext::default();
    let result = tool.call(context, args).await;
    match result {
        ::core::result::Result::Ok(output) => match ::rig::tool::IntoToolOutput::into_tool_output(output) {
            ::core::result::Result::Ok(rendered) => rendered.render(),
            ::core::result::Result::Err(error) => ::std::format!("tool error: {error}"),
        },
        ::core::result::Result::Err(error) => ::std::format!("tool error: {error}"),
    }
}

/// Wrap an internal failure into a rig prompt error. The loop surfaces
/// plain text errors as request errors.
fn request_error(error: ::std::string::String) -> ::rig::completion::PromptError {
    ::rig::completion::PromptError::CompletionError(::rig::completion::CompletionError::RequestError(
        ::std::boxed::Box::new(::std::io::Error::other(error)),
    ))
}
