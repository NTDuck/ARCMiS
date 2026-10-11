//! Command-line entrypoint for ARCMiS modernization runs.
//!
//! Run story:
//! 1. Initialize tracing subscriber to write logs to standard error.
//! 2. Parse command arguments and require one configuration file path.
//! 3. Read the configuration file from disk.
//! 4. Parse YAML and validate preflight constraints.
//! 5. Output errors or log accepted configuration.

use std::fs;
use std::io::Write;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    // WHY: tokio runtime is necessary because a later workstream wires async conductor execution.
    init_tracing();
    let arguments = std::env::args().collect::<Vec<String>>();
    let mut standard_error = std::io::stderr();
    run_with_output(&arguments, &mut standard_error)
}

/// Runs the command line workflow.
///
/// Returns process exit code.
// WHY: testable entry seam separates command arguments and standard error output from main.
pub fn run(arguments: &[String]) -> ExitCode {
    let mut standard_error = std::io::stderr();
    run_with_output(arguments, &mut standard_error)
}

/// Runs the command line workflow with a custom output writer.
pub fn run_with_output(arguments: &[String], output: &mut impl Write) -> ExitCode {
    match execute(arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            print_error(output, &error);
            ExitCode::FAILURE
        },
    }
}

fn execute(arguments: &[String]) -> Result<(), CliError> {
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
    // WHY: construction seam for mas_core::Conductor::new(config, session).
    Ok(())
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

    #[test]
    fn missing_argument_returns_failure() {
        let mut buffer = Vec::<u8>::new();
        let arguments = vec![String::from("cli")];
        let exit_code = run_with_output(&arguments, &mut buffer);

        assert_eq!(exit_code, ExitCode::FAILURE);
        let error_message = String::from_utf8(buffer).expect("valid utf8");
        assert!(error_message.contains("configs/example.yml"));
        assert!(error_message.contains("Provide one run configuration path."));
    }

    #[test]
    fn two_arguments_returns_failure() {
        let mut buffer = Vec::<u8>::new();
        let arguments = vec![String::from("cli"), String::from("first.yml"), String::from("second.yml")];
        let exit_code = run_with_output(&arguments, &mut buffer);

        assert_eq!(exit_code, ExitCode::FAILURE);
        let error_message = String::from_utf8(buffer).expect("valid utf8");
        assert!(error_message.contains("configs/example.yml"));
        assert!(error_message.contains("Provide one run configuration path."));
    }

    #[test]
    fn valid_config_returns_success() {
        let mut temp_file = tempfile::NamedTempFile::new().expect("temp file created");
        temp_file.write_all(sample_valid_yaml().as_bytes()).expect("fixture written");
        let path_string = temp_file.path().to_str().expect("valid path").to_string();

        let mut buffer = Vec::<u8>::new();
        let arguments = vec![String::from("cli"), path_string];
        let exit_code = run_with_output(&arguments, &mut buffer);

        assert_eq!(exit_code, ExitCode::SUCCESS);
        assert!(buffer.is_empty());
    }

    #[test]
    fn blank_mandatory_field_reports_every_finding() {
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
        let exit_code = run_with_output(&arguments, &mut buffer);

        assert_eq!(exit_code, ExitCode::FAILURE);
        let error_message = String::from_utf8(buffer).expect("valid utf8");
        assert!(error_message.contains("Preflight validation failed"));
        assert!(error_message.contains("run.name: must not be empty or whitespace"));
        assert!(error_message.contains("paths.input_dir: must not be empty or whitespace"));
    }

    #[test]
    fn unknown_key_reports_binding_error() {
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
        let exit_code = run_with_output(&arguments, &mut buffer);

        assert_eq!(exit_code, ExitCode::FAILURE);
        let error_message = String::from_utf8(buffer).expect("valid utf8");
        assert!(error_message.contains("YAML configuration failed to bind"));
        assert!(error_message.contains("unexpected_field"));
    }

    #[test]
    fn run_entry_seam_invokes_workflow() {
        let arguments = vec![String::from("cli")];
        let exit_code = run(&arguments);
        assert_eq!(exit_code, ExitCode::FAILURE);
    }
}
