//! Command-line entrypoint for ARCMiS modernization runs.
//!
//! Run story:
//! 1. Initialize tracing subscriber to write logs to standard error.
//! 2. Parse command arguments and require one configuration file path.
//! 3. Read the configuration file from disk.
//! 4. Parse YAML and validate preflight constraints.
//! 5. Drive one conductor run over the ompl transport.
//! 6. Output errors or log the run summary.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use mas_core::conductor::Conductor;
use mas_core::conductor::RunLimits;
use mas_core::conductor::RunSummary;
use error::ConductorError;
use gateway::Gateway;
use hierarchy::Role;
use hierarchy::TeamRegistry;
use router::Router;
use taskgraph::GraphBudgets;
use taskgraph::Kind;
use taskgraph::WorkItem;
use transport::OmpLaunchConfig;
use transport::OmpTransport;
use verdict::Verdict;

#[tokio::main]
async fn main() -> ExitCode {
    // WHY: the conductor round is async, so the binary needs a tokio runtime.
    init_tracing();
    let arguments = std::env::args().collect::<Vec<String>>();
    let mut standard_error = std::io::stderr();
    run_with_output(&arguments, &mut standard_error).await
}

/// Runs the command line workflow.
///
/// Returns process exit code.
// WHY: testable entry seam separates command arguments and standard error output from main.
pub async fn run(arguments: &[String]) -> ExitCode {
    let mut standard_error = std::io::stderr();
    run_with_output(arguments, &mut standard_error).await
}

/// Runs the command line workflow with a custom output writer.
pub async fn run_with_output(arguments: &[String], output: &mut impl Write) -> ExitCode {
    match execute(arguments).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            print_error(output, &error);
            ExitCode::FAILURE
        },
    }
}

async fn execute(arguments: &[String]) -> Result<(), CliError> {
    let config_path = parse_config_path(arguments)?;
    let yaml_text = read_config_file(config_path)?;
    let (config, findings) = validate_config(config_path, &yaml_text)?;

    if !findings.is_empty() {
        return Err(CliError::Preflight {
            path: config_path.to_string(),
            findings,
        });
    }

    tracing::info!(
        run = %config.run.name,
        path = %config_path,
        "configuration accepted"
    );
    drive(&config).await?;
    Ok(())
}

/// Drives one full run: builds every component from the config, spawns the
/// ompl session, seeds the first work item, and walks the phase machine.
///
/// Returns the run summary. A run that ends before `Done` maps to an error
/// so the exit code stays failure.
async fn drive(config: &run_config::SectionConfig) -> Result<RunSummary, CliError> {
    let registry = build_registry(config);
    let router = build_router(config)?;
    let gateway = build_gateway(config);
    let limits = build_limits(config);
    let transport = start_session(config)?;
    let mut conductor = Conductor::new(transport, registry, router, gateway, limits);
    seed_work(&mut conductor, config);

    let summary = conductor.run().await.map_err(|error| CliError::Run {
        error,
    })?;
    match summary.verdict {
        Verdict::Success => {
            tracing::info!(
                run = %config.run.name,
                phase = ?summary.final_phase,
                verdict = ?summary.verdict,
                dispatches = summary.dispatches,
                compactions = summary.compactions,
                "run complete"
            );
            Ok(summary)
        },
        Verdict::Failed {
            reason,
        } => Err(CliError::RunEnded {
            reason,
        }),
    }
}

/// Compiles the static team registry from the hierarchy section. A disabled
/// hierarchy yields the single-tier direct-dispatch registry.
fn build_registry(config: &run_config::SectionConfig) -> TeamRegistry {
    TeamRegistry::compile(config.hierarchy.enabled, &config.hierarchy.teams)
}

/// Builds the router. The laya triage agent loads only when the config
/// enables it.
fn build_router(config: &run_config::SectionConfig) -> Result<Router, CliError> {
    if !config.laya.enabled {
        return Ok(Router::keyword_only());
    }
    // WHY: the laya checkpoint is ~808 MB. Loading it pins that much memory,
    // so the triage agent constructs only for runs that enable it.
    let agent = laya::Agent::from_dir(&config.laya.checkpoint_dir).map_err(|source| CliError::TriageModel {
        dir: config.laya.checkpoint_dir.clone(),
        source,
    })?;
    Ok(Router::with_triage(agent))
}

/// Builds the tool gateway from the guard section. Deny patterns win over
/// allow patterns inside the gateway (ADR 0031).
fn build_gateway(config: &run_config::SectionConfig) -> Gateway {
    Gateway::new(config.guard.denylist.clone(), config.guard.allowlist.clone())
}

/// Builds the run budgets from the limits and compaction sections.
fn build_limits(config: &run_config::SectionConfig) -> RunLimits {
    // WHY: the config carries no per-event timeout. The conductor default
    // owns that value, so the call site adopts it instead of repeating it.
    RunLimits {
        graph: GraphBudgets {
            fanout: config.limits.fanout,
            max_repairs: config.limits.max_repairs,
            stagnation_rounds: config.limits.stagnation_rounds,
        },
        context_budget: config.compaction.threshold_tokens as usize,
        event_timeout: RunLimits::default().event_timeout,
    }
}

/// Spawns the model session under the `ompl` program.
fn start_session(config: &run_config::SectionConfig) -> Result<OmpTransport, CliError> {
    let working_dir = std::env::current_dir().map_err(|source| CliError::WorkingDir {
        source,
    })?;
    let launch = build_launch(config, working_dir);
    OmpTransport::spawn(&launch).map_err(|source| CliError::SessionStart {
        source,
    })
}

/// Assembles the launch config at the call site. Every run-specific value
/// bubbles from the run config or the process state. Approval mode,
/// environment, and subagent level keep the type defaults: an autonomous
/// run, no extra environment, and raw session events for the router.
fn build_launch(config: &run_config::SectionConfig, working_dir: PathBuf) -> OmpLaunchConfig {
    // WHY: the program is `ompl`, not `omp`. User directive: every turn must
    // run on the local SLM. `ompl` remaps every model to the ashalliant
    // engine and sits in the user PATH.
    OmpLaunchConfig {
        program: "ompl".to_string(),
        model: config.model.id.clone(),
        extra_args: config.model.extra_args.clone(),
        working_dir,
        ..OmpLaunchConfig::default()
    }
}

/// Seeds the run with the first work item, mirroring the scripted conductor
/// test seeding: one source-map item with no dependencies. The instruction
/// carries the real run facts for the analyst.
fn seed_work(conductor: &mut Conductor<OmpTransport>, config: &run_config::SectionConfig) {
    let instruction = format!(
        "map the source in {}: migrate {} to {}; verify with '{}'",
        config.paths.input_dir,
        config.languages.pair.source,
        config.languages.pair.target,
        config.languages.test_command,
    );
    conductor.add_work(
        WorkItem {
            role: Role::Analyst,
            kind: Kind::Analyze,
            instruction,
        },
        Vec::new(),
    );
    conductor.publish_context("run", &config.run.name);
}

fn parse_config_path(arguments: &[String]) -> Result<&str, CliError> {
    if arguments.len() == 2 {
        Ok(&arguments[1])
    } else {
        Err(CliError::Usage)
    }
}

fn read_config_file(path: &str) -> Result<String, CliError> {
    fs::read_to_string(path).map_err(|source| CliError::Read {
        path: path.to_string(),
        source,
    })
}

fn validate_config(
    path: &str,
    yaml_text: &str,
) -> Result<(run_config::SectionConfig, Vec<run_config::ValidationError>), CliError> {
    run_config::parse_and_validate(yaml_text).map_err(|source| CliError::Deserialize {
        path: path.to_string(),
        source,
    })
}

fn print_error(output: &mut impl Write, error: &CliError) {
    match error {
        CliError::Usage => {
            let _ = writeln!(output, "Usage: cli <config-path>");
            let _ = writeln!(output, "Example: cli configs/example.yml");
            let _ = writeln!(output, "Provide one run configuration path.");
        },
        CliError::Read {
            path,
            source,
        } => {
            let _ = writeln!(output, "Cannot read configuration file '{path}': {source}");
        },
        CliError::Deserialize {
            path,
            source,
        } => {
            let _ = writeln!(output, "YAML configuration failed to bind for '{path}': {source}");
        },
        CliError::Preflight {
            path,
            findings,
        } => {
            let _ = writeln!(output, "Preflight validation failed for '{path}':");
            for finding in findings {
                let _ = writeln!(output, "{finding}");
            }
        },
        CliError::WorkingDir {
            source,
        } => {
            let _ = writeln!(output, "Cannot determine the working directory: {source}");
        },
        CliError::TriageModel {
            dir,
            source,
        } => {
            let _ = writeln!(output, "Cannot load the triage model from '{dir}': {source}. Check laya.checkpoint_dir.");
        },
        CliError::SessionStart {
            source,
        } => {
            let _ = writeln!(output, "Cannot start the model session: {source}. Check that 'ompl' is on PATH.");
        },
        CliError::Run {
            error,
        } => match error {
            ConductorError::Transport(detail) => {
                let _ = writeln!(output, "The model session closed during the run: {detail}. Restart the run.");
            },
            ConductorError::Config(detail) => {
                let _ = writeln!(output, "The run rejected its configuration: {detail}. Correct the file and restart.");
            },
            ConductorError::Gateway(detail) => {
                let _ =
                    writeln!(output, "The gateway denied a mandatory tool call: {detail}. Adjust the guard policy.");
            },
            ConductorError::BudgetExhausted(detail) => {
                let _ = writeln!(
                    output,
                    "The run exhausted its repair budget: {detail}. Raise limits.max_repairs or narrow the input."
                );
            },
            ConductorError::RegressionExhausted(detail) => {
                let _ = writeln!(
                    output,
                    "The run could not recover from a regression: {detail}. Correct the configuration and restart."
                );
            },
        },
        CliError::RunEnded {
            reason,
        } => {
            let _ = writeln!(output, "The run ended early: {reason}.");
        },
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).with_writer(std::io::stderr).try_init();
}

#[derive(Debug)]
enum CliError {
    Usage,
    Read {
        path: String,
        source: std::io::Error,
    },
    Deserialize {
        path: String,
        source: anyhow::Error,
    },
    Preflight {
        path: String,
        findings: Vec<run_config::ValidationError>,
    },
    WorkingDir {
        source: std::io::Error,
    },
    TriageModel {
        dir: String,
        source: laya::Error,
    },
    SessionStart {
        source: transport::TransportError,
    },
    Run {
        error: ConductorError,
    },
    RunEnded {
        reason: String,
    },
}

#[cfg(test)]
// WHY: unit tests sit inside the bin crate to avoid an artificial lib.rs split for a single binary target.
mod tests {
    use super::*;

    fn sample_valid_yaml() -> &'static str {
        r#"
run:
  name: cli-unit-test-run
paths:
  input_dir: input/path
  output_dir: output/path
languages:
  pair:
    source: c
    target: rust
  test_command: cargo test
"#
    }

    fn sample_full_yaml() -> &'static str {
        r#"
run:
  name: cli-full-test-run
paths:
  input_dir: input/path
  output_dir: output/path
languages:
  pair:
    source: c
    target: rust
  test_command: cargo test
model:
  id: ashalliants/qwen3.8-27b
  extra_args: [--local]
hierarchy:
  enabled: true
  teams:
    - lead: discovery-lead
      members: [analyst, architect]
      turns: 40
      stagnation_rounds: 3
    - lead: migration-lead
      members: [translator, validator]
      turns: 40
      stagnation_rounds: 3
guard:
  allowlist: ['read:', 'bash:']
  denylist: [bash:rm -rf]
compaction:
  threshold_tokens: 12000
  keep_recent_tokens: 4000
limits:
  turns: 60
  fanout: 2
  collective_turns: 40
  max_repairs: 2
  stagnation_rounds: 3
"#
    }

    #[tokio::test]
    async fn missing_argument_returns_failure() {
        let mut buffer = Vec::<u8>::new();
        let arguments = vec![String::from("cli")];
        let exit_code = run_with_output(&arguments, &mut buffer).await;

        assert_eq!(exit_code, ExitCode::FAILURE);
        let error_message = String::from_utf8(buffer).expect("valid utf8");
        assert!(error_message.contains("configs/example.yml"));
        assert!(error_message.contains("Provide one run configuration path."));
    }

    #[tokio::test]
    async fn two_arguments_returns_failure() {
        let mut buffer = Vec::<u8>::new();
        let arguments = vec![String::from("cli"), String::from("first.yml"), String::from("second.yml")];
        let exit_code = run_with_output(&arguments, &mut buffer).await;

        assert_eq!(exit_code, ExitCode::FAILURE);
        let error_message = String::from_utf8(buffer).expect("valid utf8");
        assert!(error_message.contains("configs/example.yml"));
        assert!(error_message.contains("Provide one run configuration path."));
    }

    #[tokio::test]
    async fn valid_config_reaches_execution() {
        let mut temp_file = tempfile::NamedTempFile::new().expect("temp file created");
        temp_file.write_all(sample_valid_yaml().as_bytes()).expect("fixture written");
        let path_string = temp_file.path().to_str().expect("valid path").to_string();

        // WHY: a live model session must not run inside a unit test. The
        // sandboxed PATH hides `ompl`, so the spawn fails before any model
        // turn runs and the test asserts that boundary deterministically.
        let saved_path = std::env::var("PATH").unwrap_or_default();
        let sandbox = tempfile::tempdir().expect("sandbox created");
        std::env::set_var("PATH", sandbox.path());

        let mut buffer = Vec::<u8>::new();
        let arguments = vec![String::from("cli"), path_string];
        let exit_code = run_with_output(&arguments, &mut buffer).await;

        std::env::set_var("PATH", &saved_path);
        drop(sandbox);

        assert_eq!(exit_code, ExitCode::FAILURE);
        let error_message = String::from_utf8(buffer).expect("valid utf8");
        assert!(error_message.contains("Cannot start the model session"));
    }

    #[tokio::test]
    async fn blank_mandatory_field_reports_every_finding() {
        let invalid_yaml = r#"
run:
  name: ""
paths:
  input_dir: ""
  output_dir: output/path
languages:
  pair:
    source: c
    target: rust
  test_command: cargo test
"#;
        let mut temp_file = tempfile::NamedTempFile::new().expect("temp file created");
        temp_file.write_all(invalid_yaml.as_bytes()).expect("fixture written");
        let path_string = temp_file.path().to_str().expect("valid path").to_string();

        let mut buffer = Vec::<u8>::new();
        let arguments = vec![String::from("cli"), path_string];
        let exit_code = run_with_output(&arguments, &mut buffer).await;

        assert_eq!(exit_code, ExitCode::FAILURE);
        let error_message = String::from_utf8(buffer).expect("valid utf8");
        assert!(error_message.contains("Preflight validation failed"));
        assert!(error_message.contains("run.name: must not be empty or whitespace"));
        assert!(error_message.contains("paths.input_dir: must not be empty or whitespace"));
    }

    #[tokio::test]
    async fn unknown_key_reports_binding_error() {
        let invalid_yaml = r#"
run:
  name: test-run
  unexpected_field: 42
paths:
  input_dir: input/path
  output_dir: output/path
languages:
  pair:
    source: c
    target: rust
  test_command: cargo test
"#;
        let mut temp_file = tempfile::NamedTempFile::new().expect("temp file created");
        temp_file.write_all(invalid_yaml.as_bytes()).expect("fixture written");
        let path_string = temp_file.path().to_str().expect("valid path").to_string();

        let mut buffer = Vec::<u8>::new();
        let arguments = vec![String::from("cli"), path_string];
        let exit_code = run_with_output(&arguments, &mut buffer).await;

        assert_eq!(exit_code, ExitCode::FAILURE);
        let error_message = String::from_utf8(buffer).expect("valid utf8");
        assert!(error_message.contains("YAML configuration failed to bind"));
        assert!(error_message.contains("unexpected_field"));
    }

    #[tokio::test]
    async fn run_entry_seam_invokes_workflow() {
        let arguments = vec![String::from("cli")];
        let exit_code = run(&arguments).await;
        assert_eq!(exit_code, ExitCode::FAILURE);
    }

    #[test]
    fn config_maps_to_component_inputs() {
        let (config, findings) = run_config::parse_and_validate(sample_full_yaml()).expect("fixture binds");
        assert!(findings.is_empty());

        let registry = build_registry(&config);
        assert_eq!(registry.teams().len(), 2);
        assert!(!registry.is_direct());

        let limits = build_limits(&config);
        assert_eq!(limits.graph.fanout, 2);
        assert_eq!(limits.graph.max_repairs, 2);
        assert_eq!(limits.graph.stagnation_rounds, 3);
        assert_eq!(limits.context_budget, 12_000);

        let gateway_instance = build_gateway(&config);
        let denied = gateway_instance.judge(&gateway::ToolCall {
            tool: String::from("bash"),
            args: String::from("rm -rf /"),
        });
        assert!(matches!(denied, gateway::ToolVerdict::Deny { .. }));
        let allowed = gateway_instance.judge(&gateway::ToolCall {
            tool: String::from("read"),
            args: String::from("one file"),
        });
        assert!(matches!(allowed, gateway::ToolVerdict::Allow));
    }

    #[test]
    fn disabled_hierarchy_maps_to_direct_registry() {
        let (config, findings) = run_config::parse_and_validate(sample_valid_yaml()).expect("fixture binds");
        assert!(findings.is_empty());

        let registry = build_registry(&config);
        assert!(registry.is_direct());
        assert_eq!(registry.teams().len(), 0);
    }

    #[test]
    fn launch_config_carries_call_site_values() {
        let (config, findings) = run_config::parse_and_validate(sample_full_yaml()).expect("fixture binds");
        assert!(findings.is_empty());

        let working_dir = PathBuf::from("/run/root");
        let launch = build_launch(&config, working_dir.clone());
        assert_eq!(launch.program, "ompl");
        assert_eq!(launch.model.as_deref(), Some("ashalliants/qwen3.8-27b"));
        assert_eq!(launch.extra_args, vec![String::from("--local")]);
        assert_eq!(launch.working_dir, working_dir);
    }
}
