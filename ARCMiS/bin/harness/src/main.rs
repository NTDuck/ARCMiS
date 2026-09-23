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
//!    over the configured fix-round ceiling (`run.recode.max_rounds`).
//! 7. Write the result yaml into `{output_dir}/.ARCMiS/result/` and log
//!    its path.
//!
//! Every agent boundary carries a typed artifact. No prompt-side coercion:
//! the rig output schemas enforce the shapes: tool calls under
//! `OutputMode::Tool`, prompted JSON under `OutputMode::Prompted`.
//!
//! Exit code: success iff the validator reports `pass`.

mod experiment;
mod offload;
mod result;
mod run_log;
use agents::util::provider::{Clients, Provider};
use agents::util::sources;
use agents::{Config, MonolithRequest, ValidatorRequest, ValidatorResponse};
use anyhow::{Context, Result};
use futures::FutureExt;
use run_log::Tracing;
use std::env::args;
use std::fs::create_dir_all;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

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

/// The harness command line. Positional config path and experiment dir
/// keep the existing contract; flags select the run knobs. Hand-rolled
/// parsing keeps the binary dependency-free, matching the repo pattern.
#[derive(Debug, Default)]
struct Args {
    config_path: Option<PathBuf>,
    experiment_dir: Option<PathBuf>,
    method: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    offload: bool,
}

/// Parse `--flag value` pairs and the two positionals.
fn parse_args() -> Args {
    let mut parsed = Args::default();
    let mut argv = args().skip(1);
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--method" => parsed.method = argv.next(),
            "--model" => parsed.model = argv.next(),
            "--provider" => parsed.provider = argv.next(),
            "--api-key" => parsed.api_key = argv.next(),
            "--base-url" => parsed.base_url = argv.next(),
            "--offload" => parsed.offload = true,
            other if parsed.config_path.is_none() => parsed.config_path = Some(PathBuf::from(other)),
            other if parsed.experiment_dir.is_none() => parsed.experiment_dir = Some(PathBuf::from(other)),
            other => {
                tracing::warn!(arg = other, "unrecognized argument ignored");
            },
        }
    }
    parsed
}

/// Sequence one migration run. Every failure propagates as an error. The
/// entry point logs it once.
async fn run() -> Result<ExitCode> {
    let args = parse_args();
    let config_path = args
        .config_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("assets/configs/GildedRose-Refactoring-Kata/config.yml"));
    // Optional second positional: the experiment directory. When present,
    // the run writes manifest, result, and trace into it.
    let experiment_dir = args.experiment_dir.clone();

    let started = Instant::now();
    let mut config =
        Config::load(&config_path).with_context(|| format!("config load failed for {}", config_path.display()))?;
    if let Some(model) = &args.model {
        config.run.model = model.clone();
    }
    // The method comes from --method, or from the config directory name
    // when the flag is absent. Unknown names run the monolith sequence.
    let method = args.method.clone().unwrap_or_else(|| {
        config_path
            .parent()
            .and_then(Path::parent)
            .and_then(|dir| dir.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned()
    });
    let log = match &experiment_dir {
        Some(dir) => {
            let trace = dir.join("traces").join("turns.jsonl");
            run_log::ensure_parent(&trace);
            run_log::RunLog::with_trace(trace)
        },
        None => run_log::RunLog::default(),
    };
    if let Some(dir) = &experiment_dir {
        experiment::write_manifest(dir, &config, &config_path, &method)?;
    }

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

    let provider =
        Provider::from_cli(args.provider, args.api_key, args.base_url).context("provider resolution failed")?;

    // Offload before the run: leftovers from a crashed earlier run free
    // their VRAM. The model about to run may stay: ollama can preload it.
    if args.offload {
        if let Provider::Ollama {
            base_url,
        } = &provider
        {
            offload::unload_all(base_url.as_deref(), Some(&config.run.model)).await.context("model offload failed")?;
        }
    }

    let client = provider.client().context("client construction failed")?;

    let task = MonolithTask::build(&config)?;

    // Monomorphic dispatch: each provider variant gets its own concrete
    // client; there is no type erasure between them.
    let method_ref = method.as_str();
    let run = match (method_ref, &client) {
        ("ledger-method", Clients::Ollama(client)) | ("ledger", Clients::Ollama(client)) => {
            run_ledger(client, &config, &provider, &task, &log, started, experiment_dir.as_deref()).boxed()
        },
        ("ReCodeAgent-method", Clients::Ollama(client)) | ("recode", Clients::Ollama(client)) => {
            run_recode(client, &config, &provider, &task, &log, started, experiment_dir.as_deref()).boxed()
        },
        (_, Clients::Ollama(client)) => {
            run_monolith(client, &config, &provider, &task, &log, started, experiment_dir.as_deref()).boxed()
        },
        ("ledger-method", Clients::Netmind(client)) | ("ledger", Clients::Netmind(client)) => {
            run_ledger(client, &config, &provider, &task, &log, started, experiment_dir.as_deref()).boxed()
        },
        ("ReCodeAgent-method", Clients::Netmind(client)) | ("recode", Clients::Netmind(client)) => {
            run_recode(client, &config, &provider, &task, &log, started, experiment_dir.as_deref()).boxed()
        },
        (_, Clients::Netmind(client)) => {
            run_monolith(client, &config, &provider, &task, &log, started, experiment_dir.as_deref()).boxed()
        },
    };
    // Offload after the run, on success and failure alike: the model is
    // resident either way, and the survey switches models between runs.
    // Failure here never changes the run's own exit code.
    let result = run.await;
    if args.offload {
        if let Provider::Ollama {
            base_url,
        } = &provider
        {
            if let Err(error) = offload::unload_all(base_url.as_deref(), None).await {
                tracing::warn!(error = %error, "post-run model offload failed");
            }
        }
    }
    result
}

/// Wire and run the monolith, then validate its output with the validator
/// agent. The original two-agent sequence.
async fn run_monolith<C>(
    client: &C,
    config: &Config,
    provider: &Provider,
    task: &MonolithRequest,
    log: &run_log::RunLog,
    started: Instant,
    experiment_dir: Option<&std::path::Path>,
) -> Result<ExitCode>
where
    C: rig::client::AgentClientExt,
    C::CompletionModel: 'static,
{
    // The resilience hook caps output tokens, retries truncated turns,
    // and rewrites tool failures; the run log underneath records it all.
    let ollama = matches!(provider, Provider::Ollama { .. });
    let hook =
        agents::util::resilience::ResilienceHook::for_provider(log.clone(), config.run.max_output_tokens, ollama);
    let monolith = agents::Monolith::build(client, config, provider, hook.clone());
    let monolith_result = agents::Monolith::run(&monolith, task, config.run.max_turns, config.run.max_retries).await?;
    tracing::info!(
        files_written = monolith_result.files_written,
        approach = %monolith_result.approach,
        "monolith finished"
    );

    // The shared validator runs at the end of every method.
    let validation =
        run_validator(client, config, provider, &monolith_result.output_dir, &monolith_result.approach, &hook).await?;
    result::finish(config, &validation, started.elapsed(), experiment_dir)
}

/// Wire and run the shared validator agent over a produced workspace.
/// Every method ends in this typed report.
async fn run_validator<C>(
    client: &C,
    config: &Config,
    provider: &Provider,
    output_dir: &str,
    approach: &str,
    hook: &agents::util::resilience::ResilienceHook<run_log::RunLog>,
) -> Result<ValidatorResponse>
where
    C: rig::client::AgentClientExt,
    C::CompletionModel: 'static,
{
    let validation_task = ValidatorRequest {
        output_dir: output_dir.to_owned(),
        toolchain: vec!["build".to_owned(), "test".to_owned()],
        test_command: config.source.target.test_command.clone(),
        approach: approach.to_owned(),
    };
    let validator = agents::Validator::build(client, config, provider, hook.clone());
    let validation =
        agents::Validator::run(&validator, &validation_task, config.run.max_turns, config.run.max_retries).await?;
    tracing::info!(
        compiled = validation.compiled,
        test_pass_rate = ?validation.test_pass_rate,
        "validator finished"
    );
    Ok(validation)
}

/// Wire and run the ledger method, then validate its output with the
/// shared validator. The manager works through fresh-budget worker
/// delegations; the harness-side verdict comes from the validator agent,
/// mirroring the paper's harness-side public-test verifier.
async fn run_ledger<C>(
    client: &C,
    config: &Config,
    provider: &Provider,
    task: &MonolithRequest,
    log: &run_log::RunLog,
    started: Instant,
    experiment_dir: Option<&std::path::Path>,
) -> Result<ExitCode>
where
    C: rig::client::AgentClientExt,
    C::CompletionModel: 'static,
{
    let ollama = matches!(provider, Provider::Ollama { .. });
    let hook =
        agents::util::resilience::ResilienceHook::for_provider(log.clone(), config.run.max_output_tokens, ollama);
    let ledger = agents::Ledger::build(client, config, provider, hook.clone(), config.ledger.clone());
    let result = agents::Ledger::run(&ledger, task, config.ledger.manager_turns, config.run.max_retries).await?;
    tracing::info!(
        compiled = result.compiled,
        test_pass_rate = ?result.test_pass_rate,
        approach = %result.approach,
        "ledger finished"
    );
    let validation =
        run_validator(client, config, provider, &config.output.dir.to_string_lossy(), &result.approach, &hook).await?;
    result::finish(config, &validation, started.elapsed(), experiment_dir)
}

/// Wire and run the ReCode method, then validate its output with the
/// shared validator. The Recode namespace builds its four agents
/// (analyzer, planner, translator, validator) and runs the deterministic
/// pipeline from the paper's Algorithm 1 with per-phase budgets. The
/// final shared validator produces the harness-side typed report.
async fn run_recode<C>(
    client: &C,
    config: &Config,
    provider: &Provider,
    task: &MonolithRequest,
    log: &run_log::RunLog,
    started: Instant,
    experiment_dir: Option<&std::path::Path>,
) -> Result<ExitCode>
where
    C: rig::client::AgentClientExt,
    C::CompletionModel: 'static,
{
    let ollama = matches!(provider, Provider::Ollama { .. });
    let hook =
        agents::util::resilience::ResilienceHook::for_provider(log.clone(), config.run.max_output_tokens, ollama);
    let budgets = agents::recode::PhaseBudgets::uniform(config.run.max_turns, config.run.max_retries);
    let result =
        agents::Recode::run(client, config, provider, task, hook.clone(), budgets, config.recode.max_rounds).await?;
    tracing::info!(
        compiled = result.compiled,
        test_pass_rate = ?result.test_pass_rate,
        repos_translated = result.repos_translated,
        files_written = result.files_written,
        approach = %result.approach,
        "recode finished"
    );
    let validation =
        run_validator(client, config, provider, &config.output.dir.to_string_lossy(), &result.approach, &hook).await?;
    result::finish(config, &validation, started.elapsed(), experiment_dir)
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
