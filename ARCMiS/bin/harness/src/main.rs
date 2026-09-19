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
//!    The `ReCodeAgent-method` config selects the ReCode pipeline: a
//!    four-agent deterministic loop (analyze, plan, translate, validate)
//!    over `MAX_ITER` outer iterations.
//! 7. Write the result yaml into `{output_dir}/.ARCMiS/result/` and log
//!    its path.
//!
//! Every agent boundary carries a typed artifact. No prompt-side coercion:
//! the rig output schemas enforce the shapes: tool calls under
//! `OutputMode::Tool`, prompted JSON under `OutputMode::Prompted`.
//!
//! Exit code: success iff the validator reports `pass`.

mod result;
mod run_log;

use agents::util::sources;
use agents::{Config, MonolithRequest, ValidatorRequest, ValidatorResponse};
use anyhow::{Context, Result};
use rig::client::ProviderClient;
use rig::providers::ollama::Client;
use run_log::Tracing;
use std::env::args;
use std::fs::create_dir_all;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

/// Algorithm 1 maximum outer iterations for the ReCode method.
const MAX_ITER: usize = 5;

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

/// Sequence one migration run. Every failure propagates as an error. The
/// entry point logs it once.
async fn run() -> Result<ExitCode> {
    let args = args().collect::<Vec<String>>();
    let config_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("assets/configs/GildedRose-Refactoring-Kata/config.yml"));

    let started = Instant::now();
    let config =
        Config::load(&config_path).with_context(|| format!("config load failed for {}", config_path.display()))?;

    // The output dir must exist before the agents write into it. The
    // package structure itself is the monolith's job. No scaffold here.
    create_dir_all(&config.output.dir)
        .with_context(|| format!("output dir create failed at {}", config.output.dir.display()))?;
    tracing::info!(
        model = %config.run.model,
        max_turns = config.run.max_turns,
        num_ctx = config.run.num_ctx,
        output_dir = %config.output.dir.display(),
        "migration run started"
    );

    // One ollama client, constructed in run, shared by both agents.
    let client = Client::from_env().context("ollama client construction failed")?;

    let task = MonolithTask::build(&config)?;

    // The method comes from the config directory name. Unknown names run
    // the monolith sequence.
    let method = config_path
        .parent()
        .and_then(Path::parent)
        .and_then(|dir| dir.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();
    match method.as_str() {
        "ledger-method" => run_ledger(&client, &config, &task, started).await,
        "ReCodeAgent-method" => run_recode(&client, &config, &task, started).await,
        _ => run_monolith(&client, &config, &task, started).await,
    }
}

/// Wire and run the monolith, then validate its output with the validator
/// agent. The original two-agent sequence.
async fn run_monolith(client: &Client, config: &Config, task: &MonolithRequest, started: Instant) -> Result<ExitCode> {
    // The shared hook logs every turn and tool call to the console.
    let monolith = agents::Monolith::build(client, config, run_log::RunLog);
    let monolith_result = agents::Monolith::run(&monolith, task, config.run.max_turns).await?;
    tracing::info!(
        files_written = monolith_result.files_written,
        approach = %monolith_result.approach,
        "monolith finished"
    );

    // Wire and run the validator over the monolith's output.
    let validation_task = ValidatorRequest {
        output_dir: monolith_result.output_dir.clone(),
        toolchain: vec!["build".to_owned(), "test".to_owned()],
        test_command: config.source.target.test_command.clone(),
        approach: monolith_result.approach.clone(),
    };
    let validator = agents::Validator::build(client, config, run_log::RunLog);
    let validation = agents::Validator::run(&validator, &validation_task, config.run.max_turns).await?;
    tracing::info!(
        compilation_status = %validation.compilation_status,
        test_pass_rate = ?validation.test_pass_rate,
        "validator finished"
    );

    result::finish(config, &validation, started.elapsed())
}

/// Wire and run the ledger method. The manager reports its own validation,
/// so no second agent runs. The result lands in the same yaml shape.
async fn run_ledger(client: &Client, config: &Config, task: &MonolithRequest, started: Instant) -> Result<ExitCode> {
    let ledger = agents::Ledger::build(client, config, run_log::RunLog);
    let result = agents::Ledger::run(&ledger, task, config.run.max_turns).await?;
    tracing::info!(
        compilation_status = %result.compilation_status,
        test_pass_rate = ?result.test_pass_rate,
        approach = %result.approach,
        "ledger finished"
    );
    let validation = ValidatorResponse {
        compilation_status: result.compilation_status,
        test_pass_rate: result.test_pass_rate,
        steps: Vec::new(),
    };
    result::finish(config, &validation, started.elapsed())
}

/// Wire and run the ReCode method. The Recode namespace builds its four
/// agents (analyzer, planner, translator, validator) and runs the
/// deterministic pipeline from the paper's Algorithm 1. The final reporter
/// reports its own validation, so no second agent runs. The result lands
/// in the same yaml shape.
async fn run_recode(client: &Client, config: &Config, task: &MonolithRequest, started: Instant) -> Result<ExitCode> {
    let result = agents::Recode::run(client, config, task, run_log::RunLog, config.run.max_turns, MAX_ITER).await?;
    tracing::info!(
        compilation_status = %result.compilation_status,
        test_pass_rate = ?result.test_pass_rate,
        repos_translated = result.repos_translated,
        files_written = result.files_written,
        approach = %result.approach,
        "recode finished"
    );
    let validation = ValidatorResponse {
        compilation_status: result.compilation_status,
        test_pass_rate: result.test_pass_rate,
        steps: Vec::new(),
    };
    result::finish(config, &validation, started.elapsed())
}

/// The structured monolith task. Discovers the input sources and bundles
/// them with the target toolchain from the config.
struct MonolithTask;

impl MonolithTask {
    /// Discover the sources and build the request.
    fn build(config: &Config) -> Result<MonolithRequest> {
        // One quarter of the context window for one file. The first `4` is
        // the quarter. The second `4` is bytes per token. Tokens average
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
