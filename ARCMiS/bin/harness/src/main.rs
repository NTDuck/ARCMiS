//! Binary entry point for ARCMiS. The harness is a thin sequencer:
//!
//! 1. Install the tracing subscriber (env filter, default level `info`).
//! 2. Load the run config from the path in `argv[1]` (default path below).
//! 3. Construct the ollama client (base URL from `OLLAMA_API_BASE_URL`).
//! 4. Discover inputs and build the structured [`MonolithRequest`].
//! 5. Wire the monolith agent (tools: `write`, `bash` in the output root)
//!    and run it. The run streams through the [`RunLog`] hook to the
//!    console. The monolith returns a structured [`agents::MonolithResponse`].
//! 6. Wire the validator agent (tool: `bash` in the output root), pass it
//!    the structured [`ValidatorRequest`], and run it with the same hook. The
//!    validator returns a structured [`ValidatorResponse`].
//! 7. Write the result yaml into `{output_dir}/.ARCMiS/result/` and log
//!    its path.
//!
//! Every agent boundary carries a typed artifact. No prompt-side coercion:
//! the rig output schemas (`OutputMode::Tool`) enforce the shapes.
//!
//! Exit code: success iff the validator reports `pass`.

use agents::util::sources;
use agents::{Config, MonolithRequest, ValidatorRequest, ValidatorResponse, ValidatorStepOutcome};

use anyhow::{Context, Result};
use serde::Serialize;
use time::OffsetDateTime;

use rig::agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, ToolCall, ToolCallAction};
use rig::client::ProviderClient;

use std::process::ExitCode;
use std::time::{Duration, Instant};

/// Tool-call args preview length. Longer args are cut and marked.
const ARG_PREVIEW_CHARS: usize = 200;

#[tokio::main]
async fn main() -> ExitCode {
    Tracing::init();
    match run().await {
        Ok(code) => code,
        Err(error) => {
            tracing::error!(error = %error, "harness run failed");
            ExitCode::FAILURE
        },
    }
}

/// Sequence one migration run. Every failure propagates as an error; the
/// entry point logs it once.
async fn run() -> Result<ExitCode> {
    let args = std::env::args().collect::<Vec<String>>();
    let config_path = args
        .get(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("assets/configs/GildedRose-Refactoring-Kata/config.yml"));

    let started = Instant::now();
    let config =
        Config::load(&config_path).with_context(|| format!("config load failed for {}", config_path.display()))?;

    // The output dir must exist before the agents write into it. The
    // package structure itself is the monolith's job; no scaffold here.
    std::fs::create_dir_all(&config.output.dir)
        .with_context(|| format!("output dir create failed at {}", config.output.dir.display()))?;
    tracing::info!(
        model = %config.run.model,
        max_turns = config.run.max_turns,
        num_ctx = config.run.num_ctx,
        output_dir = %config.output.dir.display(),
        "migration run started"
    );

    // One ollama client, constructed in run, shared by both agents.
    let client = rig::providers::ollama::Client::from_env().context("ollama client construction failed")?;

    let task = MonolithTask::build(&config)?;

    // Wire and run the monolith. The shared hook logs every turn and tool
    // call to the console.
    let monolith = agents::build_monolith(&client, &config, RunLog);
    let monolith_result = agents::run_monolith(&monolith, &task, config.run.max_turns).await?;
    tracing::info!(
        files_written = monolith_result.files_written,
        approach = %monolith_result.approach,
        "monolith finished"
    );

    // Wire and run the validator over the monolith's output.
    let validation_task = ValidatorRequest {
        output_dir: monolith_result.output_dir.clone(),
        toolchain: vec![String::from("build"), String::from("test")],
        test_command: config.source.target.test_command.clone(),
        approach: monolith_result.approach.clone(),
    };
    let validator = agents::build_validator(&client, &config, RunLog);
    let validation = agents::run_validator(&validator, &validation_task, config.run.max_turns).await?;
    tracing::info!(
        compilation_status = %validation.compilation_status,
        test_pass_rate = ?validation.test_pass_rate,
        "validator finished"
    );

    RunResult::write(&config, &validation, started.elapsed())?;

    if validation.compilation_status == "pass" {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

/// The tracing subscriber. One global install with an env filter.
struct Tracing;

impl Tracing {
    /// Install the subscriber. `RUST_LOG` selects the filter; `info` is the default.
    fn init() {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    }
}

/// The structured monolith task. Discovers the input sources and bundles
/// them with the target toolchain from the config.
struct MonolithTask;

impl MonolithTask {
    /// Discover the sources and build the request.
    fn build(config: &Config) -> Result<MonolithRequest> {
        // One quarter of the context window for one file. The first `4` is
        // the quarter; the second `4` is bytes per token. Tokens average
        // 4 bytes of source text, so the byte cap is tokens divided by 4.
        let per_file_cap = config.run.num_ctx / 4 * 4;
        let sources = sources::collect(&config.source.root, per_file_cap)?;
        Ok(MonolithRequest {
            sources,
            source_language: config.source.language.clone(),
            target_language: config.source.target.language.clone(),
            test_command: config.source.target.test_command.clone(),
        })
    }
}

/// The run log. Tool calls surface through the rig hook; the agents'
/// structured results land in the tracing log and the result yaml.
#[derive(Clone, Default)]
struct RunLog;

impl AgentHook for RunLog {
    async fn on_completion_call(&self, ctx: &HookContext, _event: CompletionCallEvent<'_>) -> CompletionCallAction {
        tracing::info!(turn = ctx.turn(), "model call");
        CompletionCallAction::Continue
    }

    async fn on_tool_call(&self, ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        tracing::info!(
            turn = ctx.turn(),
            tool = event.tool_name,
            args = %truncate_args(event.args),
            "tool call"
        );
        ToolCallAction::Run
    }
}

/// One run's recorded result. Rendered as yaml into the output dir.
struct RunResult;

impl RunResult {
    /// Render the validation result as yaml and write it to
    /// `{output_dir}/.ARCMiS/result/{timestamp}.yml`.
    fn write(config: &Config, validation: &ValidatorResponse, elapsed: Duration) -> Result<()> {
        let record = RunRecord {
            compilation_status: validation.compilation_status.clone(),
            test_pass_rate: fmt_pass_rate(validation.test_pass_rate),
            elapsed: format!("{elapsed:?}"),
            steps: validation
                .steps
                .iter()
                .map(|step| RunStepRecord {
                    step: step.step.clone(),
                    passed: step.passed,
                    detail: step_report_line(step),
                })
                .collect(),
        };
        // Dotdir prefix keeps the result out of the translated codebase listing.
        let result_dir = config.output.dir.join(".ARCMiS").join("result");
        std::fs::create_dir_all(&result_dir)
            .with_context(|| format!("result dir create failed at {}", result_dir.display()))?;
        let timestamp = OffsetDateTime::now_utc()
            .format(&time::macros::format_description!("[year][month][day]T[hour][minute][second]Z"))
            .context("run result timestamp format failed")?;
        let result_path = result_dir.join(format!("{timestamp}.yml"));
        std::fs::write(&result_path, serde_yaml::to_string(&record)?)
            .with_context(|| format!("run result write failed at {}", result_path.display()))?;
        tracing::info!(
            result = %result_path.display(),
            compilation_status = %validation.compilation_status,
            test_pass_rate = ?validation.test_pass_rate,
            elapsed = ?elapsed,
            "run result written"
        );
        Ok(())
    }
}

/// The yaml document for one run.
#[derive(Serialize)]
struct RunRecord {
    compilation_status: String,
    test_pass_rate: String,
    elapsed: String,
    steps: Vec<RunStepRecord>,
}

/// One toolchain step in the yaml document.
#[derive(Serialize)]
struct RunStepRecord {
    step: String,
    passed: bool,
    detail: String,
}

/// Cut `args` to `ARG_PREVIEW_CHARS` characters and append `...` when cut.
fn truncate_args(args: &str) -> String {
    if args.chars().count() <= ARG_PREVIEW_CHARS {
        return String::from(args);
    }
    let cut: String = args.chars().take(ARG_PREVIEW_CHARS).collect();
    format!("{cut}...")
}

/// Render one toolchain step outcome as a detail line. Test counts appear
/// when the step reported them.
fn step_report_line(step: &ValidatorStepOutcome) -> String {
    match (step.tests_passed, step.tests_failed) {
        (Some(passed), Some(failed)) => format!("tests passed {passed}, failed {failed}"),
        _ => String::new(),
    }
}

/// Render the pass rate for the record: `n/a` when no test step reported.
fn fmt_pass_rate(rate: Option<f64>) -> String {
    match rate {
        Some(rate) => format!("{rate:.2}"),
        None => String::from("n/a"),
    }
}
