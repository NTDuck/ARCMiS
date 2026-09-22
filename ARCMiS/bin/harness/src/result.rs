//! One run's recorded result, rendered as yaml into the output dir.
//!
//! The three run methods converge on one finish path: build the
//! validator-shaped record, write the yaml, map the validation outcome
//! to the process exit code.

use agents::{Config, ValidatorResponse};
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::create_dir_all;
use std::time::Duration;
use time::OffsetDateTime;

use crate::run_log::step_report_line;

/// One run's recorded result. Rendered as yaml into the output dir.
pub struct RunResult;

impl RunResult {
    /// Render the validation result as yaml and write it to
    /// `{output_dir}/.ARCMiS/result/{timestamp}.yml`.
    pub fn write(config: &Config, validation: &ValidatorResponse, elapsed: Duration) -> Result<()> {
        let record = RunRecord {
            compiled: validation.compiled,
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
        create_dir_all(&result_dir).with_context(|| format!("result dir create failed at {}", result_dir.display()))?;
        let timestamp = OffsetDateTime::now_utc()
            .format(&time::macros::format_description!("[year][month][day]T[hour][minute][second]Z"))
            .context("run result timestamp format failed")?;
        let result_path = result_dir.join(format!("{timestamp}.yml"));
        std::fs::write(&result_path, serde_yaml::to_string(&record)?)
            .with_context(|| format!("run result write failed at {}", result_path.display()))?;
        tracing::info!(
            result = %result_path.display(),
            compiled = validation.compiled,
            test_pass_rate = ?validation.test_pass_rate,
            elapsed = ?elapsed,
            "run result written"
        );
        Ok(())
    }
}

/// Record the validation result and map the outcome to the exit code.
/// When `experiment` is present, the run also writes the per-problem
/// record and copies the aggregate yaml into the experiment directory.
pub fn finish(
    config: &Config,
    validation: &ValidatorResponse,
    elapsed: Duration,
    experiment: Option<&std::path::Path>,
) -> Result<std::process::ExitCode> {
    RunResult::write(config, validation, elapsed)?;
    if let Some(dir) = experiment {
        crate::experiment::write_per_problem(dir, config)?;
        crate::experiment::copy_result(dir, config)?;
    }
    if validation.compiled {
        Ok(std::process::ExitCode::SUCCESS)
    } else {
        Ok(std::process::ExitCode::FAILURE)
    }
}

/// The yaml document for one run.
#[derive(Serialize)]
struct RunRecord {
    compiled: bool,
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

/// Render the pass rate for the record: `n/a` when no test step reported.
fn fmt_pass_rate(rate: Option<f64>) -> String {
    match rate {
        Some(rate) => format!("{rate:.2}"),
        None => "n/a".to_owned(),
    }
}
