//! Binary entry point for ARCMiS. The harness is a thin sequencer:
//!
//! 1. Install the tracing subscriber (env filter, default level `info`).
//! 2. Load the run config from the path in `argv[1]` (default path below).
//! 3. Construct the ollama client (base URL from `OLLAMA_API_BASE_URL`).
//! 4. Discover inputs and build the structured [`MonolithRequest`].
//! 5. Wire the monolith agent (tools: `write`, `bash` in the output root)
//!    and run it. The run streams through the [`RunLog`] hook to the
//!    console. The monolith returns a structured [`MonolithResponse`].
//! 6. Wire the validator agent (tool: `bash` in the output root), pass it
//!    the structured [`ValidatorRequest`], and run it with the same hook. The
//!    validator returns a structured [`ValidatorResponse`].
//! 7. Write `run-report.md` into the output dir and log its path.
//!
//! Every agent boundary carries a typed artifact. No prompt-side coercion:
//! the rig output schemas (`OutputMode::Tool`) enforce the shapes.
//!
//! Exit code: success iff the validator reports `pass`.

use ::agents::{MonolithRequest, ValidatorRequest, ValidatorStepOutcome};
use ::rig::agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, ToolCall, ToolCallAction};
use ::rig::client::ProviderClient;

use ::agents::Config;

/// Tool-call args preview length. Longer args are cut and marked.
const ARG_PREVIEW_CHARS: usize = 200;

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

    // The output dir must exist before the agents write into it. The
    // package structure itself is the monolith's job; no scaffold here.
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

    // One ollama client, constructed in main, shared by both agents.
    let client = match ::rig::providers::ollama::Client::from_env() {
        ::core::result::Result::Ok(client) => client,
        ::core::result::Result::Err(e) => {
            ::tracing::error!(error = %e, "ollama client construction failed");
            return ::std::process::ExitCode::FAILURE;
        },
    };

    let task = match monolith_task(&config) {
        ::core::result::Result::Ok(task) => task,
        ::core::result::Result::Err(e) => {
            ::tracing::error!(root = %config.source.root.display(), error = %e, "input discovery failed");
            return ::std::process::ExitCode::FAILURE;
        },
    };

    // Wire and run the monolith. The shared hook logs every turn and tool
    // call to the console.
    let monolith = ::agents::build_monolith(&client, &config, RunLog);
    let monolith_result = match ::agents::run_monolith(&monolith, &task, config.run.max_turns).await {
        ::core::result::Result::Ok(result) => result,
        ::core::result::Result::Err(e) => {
            ::tracing::error!(error = %e, "monolith run failed");
            return ::std::process::ExitCode::FAILURE;
        },
    };
    ::tracing::info!(
        files_written = monolith_result.files_written,
        approach = %monolith_result.approach,
        "monolith finished"
    );

    // Wire and run the validator over the monolith's output.
    let validation_task = ValidatorRequest {
        output_dir: monolith_result.output_dir.clone(),
        toolchain: ::std::vec![::std::string::String::from("build"), ::std::string::String::from("test")],
        test_command: config.source.target.test_command.clone(),
        approach: monolith_result.approach.clone(),
    };
    let validator = ::agents::build_validator(&client, &config, RunLog);
    let validation = match ::agents::run_validator(&validator, &validation_task, config.run.max_turns).await {
        ::core::result::Result::Ok(result) => result,
        ::core::result::Result::Err(e) => {
            ::tracing::error!(error = %e, "validator run failed");
            return ::std::process::ExitCode::FAILURE;
        },
    };
    ::tracing::info!(
        compilation_status = %validation.compilation_status,
        test_pass_rate = ?validation.test_pass_rate,
        "validator finished"
    );

    let elapsed = started.elapsed();
    report(&config, &validation, elapsed);

    if validation.compilation_status == "pass" {
        ::std::process::ExitCode::SUCCESS
    } else {
        ::std::process::ExitCode::FAILURE
    }
}

fn init_tracing() {
    ::tracing_subscriber::fmt()
        .with_env_filter(
            ::tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| ::tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}

fn load_config(path: &::std::path::Path) -> ::core::result::Result<Config, ::std::string::String> {
    Config::load(path)
}

/// Build the structured monolith task: discover the input sources and
/// bundle them with the target toolchain from the config.
fn monolith_task(config: &Config) -> ::core::result::Result<MonolithRequest, ::std::string::String> {
    let per_file_cap = config.run.num_ctx / 4 * ::agents::util::sources::BYTES_PER_TOKEN;
    let sources = ::agents::util::sources::collect(&config.source.root, per_file_cap)?;
    Ok(MonolithRequest {
        sources,
        source_language: config.source.language.clone(),
        target_language: config.source.target.language.clone(),
        test_command: config.source.target.test_command.clone(),
    })
}

/// The run log. Tool calls surface through the rig hook; the agents'
/// structured results land in the tracing log and the run report.
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

/// Write the run report from the structured validation result.
fn report(config: &Config, validation: &::agents::ValidatorResponse, elapsed: ::std::time::Duration) {
    let steps =
        validation.steps.iter().map(step_report_line).collect::<::std::vec::Vec<::std::string::String>>().join("\n");
    let record = ::std::format!(
        "# validation\ncompilation_status: {}\ntest_pass_rate: {}\nelapsed: {:?}\n\n## toolchain steps\n{}\n",
        validation.compilation_status,
        fmt_pass_rate(validation.test_pass_rate),
        elapsed,
        steps
    );
    let report_path = config.output.dir.join("run-report.md");
    match ::std::fs::write(&report_path, &record) {
        ::core::result::Result::Ok(()) => ::tracing::info!(
            report = %report_path.display(),
            compilation_status = %validation.compilation_status,
            test_pass_rate = ?validation.test_pass_rate,
            elapsed = ?elapsed,
            "run report written"
        ),
        ::core::result::Result::Err(e) => {
            ::tracing::warn!(path = %report_path.display(), error = %e, "run report write failed")
        },
    }
}

/// Render one toolchain step outcome as a report line.
fn step_report_line(step: &ValidatorStepOutcome) -> ::std::string::String {
    ::std::format!(
        "- {}: {}{}",
        step.step,
        if step.passed {
            "pass"
        } else {
            "fail"
        },
        match (step.tests_passed, step.tests_failed) {
            (::core::option::Option::Some(passed), ::core::option::Option::Some(failed)) => {
                ::std::format!(" (tests passed {passed}, failed {failed})")
            },
            _ => ::std::string::String::new(),
        }
    )
}

/// Render the pass rate for the report: `n/a` when no test step reported.
fn fmt_pass_rate(rate: ::core::option::Option<f64>) -> ::std::string::String {
    match rate {
        ::core::option::Option::Some(rate) => ::std::format!("{rate:.2}"),
        ::core::option::Option::None => ::std::string::String::from("n/a"),
    }
}
