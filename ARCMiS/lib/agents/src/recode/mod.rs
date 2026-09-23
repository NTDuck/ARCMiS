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
//! with at most `max_iter` rounds.
//!
//! Every phase agent shares one snapshot store, one write tool root, and
//! one bash root at the config output dir. The phase agents talk through
//! the workspace files and through the typed task payloads. Every phase
//! runs under the caller's hook, so the whole pipeline lands in one
//! trace. Each phase carries its own fresh turn budget, matching the
//! paper's per-agent timeout (Algorithm 1 input), and its own whole-task
//! retry budget.

use crate::monolith::MonolithRequest;
use crate::util::config::Config;
use crate::util::provider::Provider;
use rig::agent::AgentHook;
use rig::client::AgentClientExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tools::{Bash, Write};

pub mod analyzer;
pub mod planner;
pub mod reporter;
pub mod translator;
pub mod validator;

/// Per-phase turn budgets for one pipeline run. Algorithm 1 gives every
/// agent its own timeout; the turn budgets mirror that per-agent bound.
#[derive(Debug, Clone, Copy)]
pub struct PhaseBudgets {
    /// Analyzer model-call budget.
    pub analyzer: usize,
    /// Planner model-call budget.
    pub planner: usize,
    /// Translator model-call budget, per loop round.
    pub translator: usize,
    /// Validator model-call budget, per invocation (including the
    /// coverage-gap test generation pass).
    pub validator: usize,
    /// Reporter model-call budget.
    pub reporter: usize,
    /// Whole-phase task retries shared by every phase.
    pub retries: u32,
}

impl PhaseBudgets {
    /// One shared budget for every phase: the paper's single model and
    /// single per-agent timeout shrink to one turn count when the config
    /// gives no per-phase split.
    pub fn uniform(max_turns: usize, retries: u32) -> Self {
        Self {
            analyzer: max_turns,
            planner: max_turns,
            translator: max_turns,
            validator: max_turns,
            reporter: max_turns,
            retries,
        }
    }
}

/// The ReCode method namespace. [`Recode::run`] executes one task through
/// the fixed four phase pipeline.
pub struct Recode;

impl Recode {
    /// Run the ReCode pipeline over one task. Deterministic scaffold code
    /// per Algorithm 1 of the paper: analyze, plan, then translate and
    /// validate in a fix loop, then report. `hook` observes every phase
    /// agent. `budgets` carries the per-phase model-call budgets.
    /// `max_iter` bounds the fix rounds.
    pub async fn run<C>(
        client: &C,
        config: &Config,
        provider: &Provider,
        task: &MonolithRequest,
        hook: impl AgentHook + Clone + 'static,
        budgets: PhaseBudgets,
        max_iter: usize,
    ) -> anyhow::Result<RecodeResponse>
    where
        C: AgentClientExt,
        C::CompletionModel: 'static,
    {
        // Shared tool state, built once. Each phase agent gets its own
        // tool instances over the same store and the same output root.
        let snapshots = tools::SnapshotStore::new();

        // Phase 1: analyze the source and design the target.
        let analyzer = analyzer::Analyzer::build(
            client,
            config,
            provider,
            Write {
                root: config.output.dir.clone(),
                snapshots: snapshots.clone(),
            },
            Bash {
                root: config.output.dir.clone(),
            },
            hook.clone(),
        );
        let analyzer_report: analyzer::AnalyzerReport = {
            let design_prompt = serde_json::json!({ "task": task });
            crate::util::task::task(&analyzer, &design_prompt, budgets.analyzer, budgets.retries).await?
        };

        // Phase 2: plan the translation units. The planner reads the
        // design documents the analyzer wrote into the workspace.
        let planner = planner::Planner::build(
            client,
            config,
            provider,
            Write {
                root: config.output.dir.clone(),
                snapshots: snapshots.clone(),
            },
            Bash {
                root: config.output.dir.clone(),
            },
            hook.clone(),
        );
        let planning_output: planner::PlanningOutput = {
            let plan_prompt = serde_json::json!({ "task": task, "design_report": &analyzer_report });
            crate::util::task::task(&planner, &plan_prompt, budgets.planner, budgets.retries).await?
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
                provider,
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
                crate::util::task::task(&translator, &translator_prompt, budgets.translator, budgets.retries).await?;

            let validator = validator::Validator::build(
                client,
                config,
                provider,
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
                crate::util::task::task(&validator, &validator_prompt, budgets.validator, budgets.retries).await?;

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
                    crate::util::task::task(&validator, &generator_prompt, budgets.validator, budgets.retries).await?;
                validation_report = Some(coverage_report);
                continue;
            }

            validation_report = Some(report);
        }
        let validation_report = validation_report.expect("validation never ran");

        // Phase 5: emit the final typed response from the last
        // validation report.
        let reporter = reporter::Reporter::build(client, config, provider);
        let response: RecodeResponse =
            crate::util::task::task(&reporter, &validation_report, budgets.reporter, budgets.retries).await?;
        Ok(response)
    }
}

// Output artifact owned exclusively by this method. Per the
// artifact-ownership rule, the DTO lives here. Per the Stepdown Rule, it
// sits below the entry function as a secondary type that serves it. The
// input artifact is the shared [`MonolithRequest`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecodeResponse {
    /// Whether the final validation pass reports the build and tests as
    /// successful.
    pub compiled: bool,
    /// Fraction of translated tests that passed in the final validation
    /// pass. `None` when the target has no test suite.
    pub test_pass_rate: Option<f64>,
    /// Number of repositories produced (always one per run).
    pub repos_translated: u32,
    /// Number of files written across every phase.
    pub files_written: u32,
    /// One-line summary of the translation approach.
    pub approach: String,
}
