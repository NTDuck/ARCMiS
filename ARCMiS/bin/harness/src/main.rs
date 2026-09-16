//! Binary entry point for ARCMiS. Drives the translation agent over a run
//! config. Run pipeline:
//! 1. Install the tracing subscriber (env filter, default level `info`).
//! 2. Load the run config from the path in `argv[1]` (default path below).
//! 3. Ensure the output dir exists. The agent owns the whole package
//!    structure; the harness writes no scaffold.
//! 4. Discover inputs: walk `source.root` recursively, read every text
//!    file (skip files that fail `read_to_string`, cap each at a quarter
//!    of `run.num_ctx` tokens rendered as bytes),
//!    and pass the map to the agent render step.
//! 5. Run up to `max_retries + 1` attempts. Each attempt ends in a
//!    measurement of the output dir. Stop at the first compiling attempt;
//!    otherwise keep the best measurement. A turn-budget exhaustion ends
//!    one attempt, not the run: whatever the agent wrote is still
//!    measured.
//! 6. Write `run-report.md` into the output dir and log its path.
//!
//! Exit code: success iff the best measurement compiled.

use ::rig::agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, ToolCall, ToolCallAction};
use ::rig::completion::{Prompt, PromptError};

use ::agents::config::Config;
use ::agents::measure::Measurement;
use ::agents::sources::Sources;

/// Tool-call args preview length. Longer args are cut and marked.
const ARG_PREVIEW_CHARS: usize = 200;

/// The run log. Tool calls surface through the rig hook; the model's final
/// text and the measurement land in the tracing log and the run report.
#[derive(::core::clone::Clone, ::core::default::Default)]
struct RunLog;

impl AgentHook for RunLog {
    async fn on_completion_call(&self, ctx: &HookContext, _event: CompletionCallEvent<'_>) -> CompletionCallAction {
        ::tracing::info!(turn = ctx.turn(), "model call");
        CompletionCallAction::Continue
    }

    async fn on_tool_call(&self, ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        ::tracing::info!(
            turn = ctx.turn(),
            tool = event.tool_name,
            args = %truncate_args(event.args),
            "tool call"
        );
        ToolCallAction::Run
    }
}

/// Cut `args` to `ARG_PREVIEW_CHARS` characters and append `...` when cut.
fn truncate_args(args: &str) -> ::std::string::String {
    if args.chars().count() <= ARG_PREVIEW_CHARS {
        return ::std::string::String::from(args);
    }
    let cut: ::std::string::String = args.chars().take(ARG_PREVIEW_CHARS).collect();
    ::std::format!("{cut}...")
}

/// Install the tracing subscriber once. The level comes from `RUST_LOG`;
/// the default is `info`.
fn init_tracing() {
    ::tracing_subscriber::fmt()
        .with_env_filter(
            ::tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| ::tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}

/// Load and parse the config file at `path`.
fn load_config(path: &::std::path::Path) -> ::core::result::Result<Config, ::std::string::String> {
    ::agents::config::Config::load(path)
}

/// Walk the configured input root and collect every readable text file.
/// Cap one file at a quarter of the context window, rendered at four
/// bytes per token of source text. One quarter leaves room for the
/// preamble, the other files, the transcript, and the reply.
fn discover_sources(config: &Config) -> ::core::result::Result<Sources, ::std::string::String> {
    let per_file_cap = config.run.num_ctx / 4 * ::agents::sources::BYTES_PER_TOKEN;
    ::agents::sources::collect(&config.source.root, per_file_cap)
}

/// One agent run: build the agent, render the discovered sources into the
/// task, and drive the tool loop for the configured turn budget.
async fn run_attempt(config: &Config, sources: &Sources) -> ::core::result::Result<::std::string::String, PromptError> {
    let agent = ::agents::default::build(config);
    let task = ::agents::default::prompt(config, sources).map_err(request_error)?;
    agent.prompt(task).max_turns(config.run.max_turns).add_hook(RunLog).await
}

/// Wrap an internal failure into a rig prompt error.
fn request_error(error: ::std::string::String) -> PromptError {
    PromptError::CompletionError(::rig::completion::CompletionError::RequestError(::std::boxed::Box::new(
        ::std::io::Error::other(error),
    )))
}

/// The attempt loop. A 2B model is flaky: a turn can truncate, wander, or
/// exhaust its turn budget. Each attempt ends in a measurement of the
/// output dir; the loop stops at the first compiling attempt (speed) and
/// otherwise keeps the best measurement. A budget exhaustion is an attempt
/// end, not a fatal error: whatever the agent wrote still gets measured.
/// Returns `None` when every attempt failed before a measurement.
async fn run_attempts(config: &Config, sources: &Sources) -> ::core::option::Option<Measurement> {
    let mut best: ::core::option::Option<Measurement> = ::core::option::Option::None;
    for attempt in 1..=config.run.max_retries + 1 {
        let final_text = match run_attempt(config, sources).await {
            ::core::result::Result::Ok(text) => text,
            ::core::result::Result::Err(
                e @ PromptError::MaxTurnsError {
                    ..
                },
            ) => {
                ::std::format!("run ended at the turn budget: {e}")
            },
            ::core::result::Result::Err(e) => {
                ::tracing::warn!(attempt, error = %e, "agent run failed");
                continue;
            },
        };
        ::tracing::info!(attempt, final_output = %final_text, "agent final output");

        let measurement = ::agents::measure::measure(&config.output.dir, &config.source.target.test_command).await;
        ::tracing::info!(
            attempt,
            compile = measurement.compile,
            test_pass_rate = ?measurement.test_pass_rate,
            "attempt measurement"
        );
        let done = measurement.compile == "pass";
        best = ::core::option::Option::Some(match best {
            ::core::option::Option::Some(b) if b.compile == "pass" => b,
            _ => measurement,
        });
        if done {
            break;
        }
    }
    best
}

/// Write the run report into the output dir and log its path.
fn report(config: &Config, measurement: &Measurement, elapsed: ::std::time::Duration) {
    let record = ::std::format!(
        "# measurement\ncompile: {}\ntest_pass_rate: {:?}\nelapsed: {:?}\n",
        measurement.compile,
        measurement.test_pass_rate,
        elapsed
    );
    let report_path = config.output.dir.join("run-report.md");
    match ::std::fs::write(&report_path, &record) {
        ::core::result::Result::Ok(()) => ::tracing::info!(
            report = %report_path.display(),
            compile = measurement.compile,
            test_pass_rate = ?measurement.test_pass_rate,
            elapsed = ?elapsed,
            "run report written"
        ),
        ::core::result::Result::Err(e) => {
            ::tracing::warn!(path = %report_path.display(), error = %e, "run report write failed")
        },
    }
}

#[::tokio::main]
async fn main() -> ::std::process::ExitCode {
    init_tracing();

    let args = ::std::env::args().collect::<::std::vec::Vec<::std::string::String>>();
    let config_path = args
        .get(1)
        .map(::std::path::PathBuf::from)
        .unwrap_or_else(|| ::std::path::PathBuf::from("assets/configs/GildedRose-Refactoring-Kata/config.yml"));

    let started = ::std::time::Instant::now();
    let config = match load_config(&config_path) {
        ::core::result::Result::Ok(config) => config,
        ::core::result::Result::Err(e) => {
            ::tracing::error!(config = %config_path.display(), error = %e, "config load failed");
            return ::std::process::ExitCode::FAILURE;
        },
    };

    // The output dir must exist before the agent writes into it. The
    // package structure itself is the agent's job; no scaffold here.
    if let ::core::result::Result::Err(e) = ::std::fs::create_dir_all(&config.output.dir) {
        ::tracing::error!(dir = %config.output.dir.display(), error = %e, "output dir create failed");
        return ::std::process::ExitCode::FAILURE;
    }
    ::tracing::info!(
        model = %config.run.model,
        max_turns = config.run.max_turns,
        num_ctx = config.run.num_ctx,
        output_dir = %config.output.dir.display(),
        "migration run started"
    );

    let sources = match discover_sources(&config) {
        ::core::result::Result::Ok(sources) => sources,
        ::core::result::Result::Err(e) => {
            ::tracing::error!(root = %config.source.root.display(), error = %e, "input discovery failed");
            return ::std::process::ExitCode::FAILURE;
        },
    };

    let measurement = match run_attempts(&config, &sources).await {
        ::core::option::Option::Some(m) => m,
        ::core::option::Option::None => {
            ::tracing::error!("all attempts failed before measurement");
            return ::std::process::ExitCode::FAILURE;
        },
    };

    let elapsed = started.elapsed();
    report(&config, &measurement, elapsed);

    if measurement.compile == "pass" {
        ::std::process::ExitCode::SUCCESS
    } else {
        ::std::process::ExitCode::FAILURE
    }
}
