//! ## The ReCode method
//!
//! The multi-agent pipeline of arXiv:2604.07341: ReCodeAgent translates a
//! whole repository across languages with four specialized agents. The
//! analyzer researches the source project and writes the design document.
//! The planner extracts fragments, maps names, and writes the dependency
//! ordered implementation plan. The translator executes the plan. The
//! validator runs the target toolchain and reports failures. The
//! paper fixes the pipeline order. The orchestration here is the same
//! deterministic scaffold code (Algorithm 1), not a manager agent. The
//! last two agents form an iterative translate, validate, and fix loop
//! with at most `max_iter` rounds. The reporter turns the final validation
//! report into the typed response.
//!
//! Every phase agent shares one snapshot store, one write tool root, and
//! one bash root at the config output dir. The phase agents talk through
//! the workspace files and through the typed task payloads.

use crate::monolith::MonolithRequest;
use crate::util::config::Config;
use rig::agent::AgentHook;
use rig::providers::ollama::Client;
use tools::{Bash, Write};

pub mod analyzer;
pub mod planner;
pub mod reporter;
pub mod translator;
pub mod validator;

/// The ReCode method namespace. [`Recode::run`] executes one task through
/// the fixed four phase pipeline.
pub struct Recode;

impl Recode {
    /// Run the ReCode pipeline over one task. Deterministic scaffold code
    /// per Algorithm 1 of the paper: analyze, plan, then translate and
    /// validate in a fix loop, then report. `hook` observes the two
    /// loop agents, the translator and the validator. `max_turns` bounds
    /// each agent's model-call budget. `max_iter` bounds the fix
    /// rounds.
    pub async fn run(
        client: &Client,
        config: &Config,
        task: &MonolithRequest,
        hook: impl AgentHook + Clone + 'static,
        max_turns: usize,
        max_iter: usize,
    ) -> anyhow::Result<RecodeResponse> {
        // Shared tool state, built once. Each phase agent gets its own
        // tool instances over the same store and the same output root.
        let snapshots = tools::SnapshotStore::new();

        // Phase 1: analyze the source and design the target.
        let analyzer = analyzer::Analyzer::build(
            client,
            config,
            Write {
                root: config.output.dir.clone(),
                snapshots: snapshots.clone(),
            },
            Bash {
                root: config.output.dir.clone(),
            },
        );
        let analyzer_report: analyzer::AnalyzerReport = {
            let design_prompt = serde_json::json!({ "task": task });
            crate::util::task::task(&analyzer, &design_prompt, max_turns).await?
        };

        // Phase 2: plan the translation units. The planner reads the
        // design documents the analyzer wrote into the workspace.
        let planner = planner::Planner::build(
            client,
            config,
            Write {
                root: config.output.dir.clone(),
                snapshots: snapshots.clone(),
            },
            Bash {
                root: config.output.dir.clone(),
            },
        );
        let planning_output: planner::PlanningOutput = {
            let plan_prompt = serde_json::json!({ "task": task, "design_report": &analyzer_report });
            crate::util::task::task(&planner, &plan_prompt, max_turns).await?
        };
        let plan_value = serde_json::to_value(&planning_output)?;

        // Phase 3 and 4: the translate, validate, and fix loop. The
        // first round executes the plan. Later rounds carry the previous
        // report: failures drive translator fixes, uncovered functions
        // drive validator test generation. Paper 3.5(b) closes the
        // coverage gap with a second validation pass over the new tests.
        let mut validation_report: Option<validator::ValidationReport> = None;
        for iteration in 1..=max_iter {
            // The fix round prompt: the plan on round one, the plan
            // plus the previous report on later rounds. The report
            // carries both the failures and the coverage gaps.
            let translator_prompt = if iteration == 1 {
                plan_value.clone()
            } else {
                let report = validation_report.as_ref().expect("fix round without a prior report");
                serde_json::json!({
                    "plan": &plan_value,
                    "validation_report": report,
                })
            };
            let translator = translator::Translator::build(
                client,
                config,
                Write {
                    root: config.output.dir.clone(),
                    snapshots: snapshots.clone(),
                },
                Bash {
                    root: config.output.dir.clone(),
                },
                hook.clone(),
            );
            let translator_report: translator::TranslatorReport =
                crate::util::task::task(&translator, &translator_prompt, max_turns).await?;

            let validator = validator::Validator::build(
                client,
                config,
                Write {
                    root: config.output.dir.clone(),
                    snapshots: snapshots.clone(),
                },
                Bash {
                    root: config.output.dir.clone(),
                },
                hook.clone(),
            );
            let validator_prompt = serde_json::json!({
                "translator_report": &translator_report,
                "test_command": &task.test_command,
                "test_generation": false,
            });
            let report: validator::ValidationReport =
                crate::util::task::task(&validator, &validator_prompt, max_turns).await?;

            // Full success closes the loop: the build passes, every
            // test passes, and no plan function lacks coverage.
            if report.all_success && report.uncovered_functions.is_empty() {
                validation_report = Some(report);
                break;
            }

            // Paper 3.5(b): the validator closes the coverage gap in a
            // second pass. It generates the tests for the uncovered
            // functions, runs them, and reports the updated coverage.
            // One generation pass per loop round.
            if !report.uncovered_functions.is_empty() {
                let generator_prompt = serde_json::json!({
                    "translator_report": &translator_report,
                    "test_command": &task.test_command,
                    "test_generation": true,
                    "uncovered_functions": &report.uncovered_functions,
                });
                let coverage_report: validator::ValidationReport =
                    crate::util::task::task(&validator, &generator_prompt, max_turns).await?;
                validation_report = Some(coverage_report);
                continue;
            }

            validation_report = Some(report);
        }
        let validation_report = validation_report.expect("validation never ran");

        // Phase 5: emit the final typed response from the last
        // validation report.
        let reporter = reporter::Reporter::build(client, config);
        let response: RecodeResponse = crate::util::task::task(&reporter, &validation_report, max_turns).await?;
        Ok(response)
    }
}

// Output artifact owned exclusively by this method. Per the
// artifact-ownership rule, the DTO lives here. Per the Stepdown Rule, it
// sits below the entry function as a secondary type that serves it. The
// input artifact is the shared [`MonolithRequest`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RecodeResponse {
    /// Build outcome of the translated codebase: `pass` when the build
    /// succeeds, `fail` otherwise.
    pub compilation_status: String,
    /// Fraction of translated tests that pass. `None` when the target has
    /// no test suite.
    pub test_pass_rate: Option<f64>,
    /// Number of repositories the pipeline translated. One per run.
    pub repos_translated: u32,
    /// Number of files the pipeline wrote.
    pub files_written: u32,
    /// One-line summary of the translation approach.
    pub approach: String,
}
