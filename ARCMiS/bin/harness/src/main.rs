//! Binary entry point for ARCMiS. The MAS harness driver: load config,
//! snapshot the source, write the manifest, run the orchestrator round loop
//! under the phase state machine, and emit the result aggregate.

mod cli_sink;
mod experiment;
mod observability;

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use agents::util::provider::Provider;
use agents::Fleet;
use blackboard::Budgets;
use blackboard::Ledger;
use blackboard::Manifest;
use blackboard::Phase;
use blackboard::State;
use blackboard::TaskList;
use blackboard::Workspace;
use orchestrator::loop_::OrchestratorLoop;
use orchestrator::loop_::RoundOutcome;
use orchestrator::state_machine;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(error = format!("{error:#}"), "harness failed");
            ExitCode::FAILURE
        },
    }
}

/// One harness invocation.
struct Invocation {
    config_path: PathBuf,
    experiment_dir: Option<PathBuf>,
}

/// Parse the CLI: `harness [config.yml] [--experiment <id>] [--parents <ids>]
/// [--hypothesis <text>]`.
fn parse_cli() -> anyhow::Result<Invocation> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut config_path = PathBuf::from("assets/configs/mas/config.yml");
    let mut experiment_dir = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--experiment" => {
                let id = args.get(index + 1).ok_or_else(|| anyhow::anyhow!("--experiment needs an id"))?;
                experiment_dir = Some(PathBuf::from(format!(".artifacts/experiments/{id}")));
                index += 2;
            },
            "--config" => {
                config_path =
                    PathBuf::from(args.get(index + 1).ok_or_else(|| anyhow::anyhow!("--config needs a path"))?);
                index += 2;
            },
            other => anyhow::bail!("unknown argument {other}; expected --config or --experiment"),
        }
    }
    Ok(Invocation {
        config_path,
        experiment_dir,
    })
}

async fn run() -> anyhow::Result<()> {
    let started = Instant::now();
    let invocation = parse_cli()?;
    let config = agents::Config::load(&invocation.config_path)?;

    // Provider + client.
    let provider = Provider::from_cli(
        Some(config.run.provider.clone()),
        std::env::var("NETMIND_API_KEY").ok(),
        std::env::var("NETMIND_BASE_URL").ok(),
    )?;
    let clients = provider.client()?;

    // Output layout.
    let output_dir = PathBuf::from(&config.output.dir);
    let run_dir = output_dir.join("run");
    let workspace = Workspace::new(output_dir.join("workspace"));
    std::fs::create_dir_all(&run_dir)?;

    // Snapshot the source once. A pre-populated workspace is a run-start
    // violation: the previous run's output must not leak in.
    let source_root = PathBuf::from(&config.source.root);
    let copied = workspace.snapshot_source(&source_root)?;
    cli_sink::emit("preflight", &format!("snapshot {} files from {}", copied, source_root.display()));

    // Manifest pre-run (also into the experiment dir when one is named).
    let manifest = Manifest {
        harness_id: format!("mas-{}", now_compact()),
        method: "mas".into(),
        model: config.run.model.clone(),
        budgets: Budgets {
            orchestrator_turns: config.mas.orchestrator_turns,
            worker_turns: config.mas.worker_turns,
            max_rounds: config.mas.max_rounds,
        },
        config_path: invocation.config_path.display().to_string(),
        problem_set: source_root.parent().map(|parent| parent.display().to_string()).unwrap_or_default(),
        source_language: config.source.language.clone(),
        target_language: config.source.target.language.clone(),
        git_revision: std::env::var("HARNESS_GIT_REV").unwrap_or_default(),
        parents: std::env::var("HARNESS_PARENTS")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
        hypothesis: std::env::var("HARNESS_HYPOTHESIS").unwrap_or_default(),
        created_at: now_rfc3339(),
    };
    // Preflight the model registry before any agent runs: every configured
    // model must exist on the daemon, or the run fails here with the fix.
    let models: Vec<String> = if config.mas.model_ladder.is_empty() {
        vec![config.run.model.clone()]
    } else {
        config.mas.model_ladder.clone()
    };
    provider.check_models(&models).await?;

    blackboard::manifest::write(&run_dir, &manifest)?;
    // The workspace copy is the specialists' view of the run contract:
    // target language and toolchain come from here, not from guessing.
    let workspace_meta = output_dir.join("workspace").join("meta");
    std::fs::create_dir_all(&workspace_meta)?;
    std::fs::write(workspace_meta.join("run.json"), serde_json::to_string_pretty(&manifest)?)?;
    if let Some(dir) = &invocation.experiment_dir {
        experiment::write_pre_run(dir, &manifest)?;
    }

    // Fleet: the ladder from the config. The orchestrator sits on the top
    // rung, specialists on the weakest (the fleet analyst promotes).
    let ladder = if config.mas.model_ladder.is_empty() {
        vec![config.run.model.clone()]
    } else {
        config.mas.model_ladder.clone()
    };
    let orchestrator_model = ladder.last().cloned().unwrap_or_else(|| config.run.model.clone());
    let mut fleet = Fleet {
        orchestrator_model,
        ladder: ladder.clone(),
        role_models: Default::default(),
    };
    for role in agents::Role::ALL {
        if role != agents::Role::Orchestrator {
            fleet.assign(role, ladder.first().cloned().unwrap_or_else(|| config.run.model.clone()));
        }
    }

    // Build the agents on the selected client, with each role's tool
    // allowlist rooted at the workspace. Tools resolve relative paths
    // against this root. prompts direct specialists at source/ and target/.
    let workspace_root = output_dir.join("workspace");
    let trace_sink = middleware::trace::sink(&output_dir)?;
    let mas = config.mas.clone();
    let turns_for_role = |role: agents::Role| -> usize {
        match role {
            agents::Role::Orchestrator => mas.orchestrator_turns,
            agents::Role::Validator | agents::Role::Critic | agents::Role::FleetAnalyst => mas.judge_turns,
            _ => mas.worker_turns,
        }
    };
    let mut agents_set = match &clients {
        agents::util::provider::Clients::Ollama(client) => agents::mas::registry::build(
            client,
            &fleet,
            &config.run,
            |role| tools::build_tools(&workspace_root, role.allowed_tools()),
            Some(trace_sink.clone()),
            turns_for_role,
            Some(&config.snapcompact),
            &mas.role_output_tokens,
            &mas.role_think,
        )?,
        agents::util::provider::Clients::Netmind(client) => agents::mas::registry::build(
            client,
            &fleet,
            &config.run,
            |role| tools::build_tools(&workspace_root, role.allowed_tools()),
            Some(trace_sink.clone()),
            turns_for_role,
            // Text-only completions gateways reject image blocks with 400
            // vision_disabled (observed c15b: snapcompact PNG frames on the
            // ninfer endpoint). No frames on this provider.
            None,
            &mas.role_output_tokens,
            &mas.role_think,
        )?,
    };
    // Hierarchical mode (ADR 0026): build the tier-2 lead agents beside the
    // specialists. Single-tier runs leave the map empty.
    let leads = if mas.hierarchy.enabled {
        let teams = orchestrator::hierarchy::validate(&config.mas)?;
        match &clients {
            agents::util::provider::Clients::Ollama(client) => agents::mas::registry::build_leads(
                client,
                &fleet,
                &config.run,
                &teams,
                Some(trace_sink.clone()),
                Some(&config.snapcompact),
                &mas.role_think,
            )?,
            agents::util::provider::Clients::Netmind(client) => agents::mas::registry::build_leads(
                client,
                &fleet,
                &config.run,
                &teams,
                Some(trace_sink.clone()),
                // Same text-only rule as the specialists above.
                None,
                &mas.role_think,
            )?,
        }
    } else {
        Default::default()
    };
    agents_set.attach_leads(leads);

    // Typed-decision triage (ADR 0027): build once per run when enabled, so
    // the 850 MB checkpoint loads once, not per batch. A disabled section
    // constructs the off judge; the run behavior is unchanged.
    let jev_triage = orchestrator::JevTriage::from_config(&config.mas.jev_triage);
    if jev_triage.is_enabled() {
        cli_sink::emit("preflight", "jev triage judge loaded");
    }

    // Initial state + task list.
    let state = State {
        phase: Phase::Preflight,
        phase_delegations: 0,
        phase_delegation_watermark: 0,
        current_task: None,
        current_batch: None,
        current_model: fleet.model_for(agents::Role::Orchestrator).to_owned(),
        updated_at: now_rfc3339(),
        last_transition: "preflight".into(),
    };
    blackboard::state::write(&run_dir, &state)?;
    TaskList::new(&run_dir).write(&[])?;

    // The run loop.
    let events = observability::EventLog::new(&output_dir)?;
    let mut orchestrator_loop = OrchestratorLoop {
        last_round_replanned: false,
        run_dir: run_dir.clone(),
        agents: agents_set,
        config: config.mas.clone(),
        workspace,
        ledger: Ledger::new(run_dir.join("ledgers")),
        jev_triage,
        round: 0,
        max_retries: config.run.max_retries,
    };
    let mut breaker_state = orchestrator::breaker::BreakerState::new();
    let mut delegations = 0usize;
    let mut stop_reason = None;

    // Drive: advance Preflight -> Discovery, then loop rounds.
    let mut current = blackboard::state::read(&run_dir)?.expect("state written above");
    state_machine::apply(&mut current, Phase::Discovery, "preflight passed")?;
    blackboard::state::write(&run_dir, &current)?;
    events.record("phase", serde_json::json!({"to": "Discovery"}))?;

    for _round in 0..config.mas.max_rounds {
        let progress_before = orchestrator::progress::snapshot(&run_dir)?;
        let mut productive_replans = 0usize;
        let outcome = orchestrator_loop.round().await?;
        let progress = orchestrator::progress::snapshot(&run_dir)?;

        match &outcome {
            RoundOutcome::Delegated {
                role,
                task,
                ..
            } => {
                delegations += 1;
                cli_sink::emit("delegate", &format!("{} <- {}", role.name(), task));
                events.record("delegation", serde_json::json!({"role": role.name(), "task": task}))?;
            },
            RoundOutcome::Replanned {
                plan_persisted,
            } => {
                cli_sink::emit("replan", "orchestrator rewrote the plan");
                // A replan that actually persisted a plan body is
                // productive orchestrator work, not a stalled round
                // (v3r3: three honest replans tripped the stagnation
                // breaker). Without a persisted body the round still
                // counts as stalled.
                if *plan_persisted {
                    productive_replans += 1;
                }
            },
            RoundOutcome::Escalated(reason) => {
                cli_sink::emit("escalate", reason);
                events.record("escalate", serde_json::json!({"reason": reason}))?;
            },
            RoundOutcome::PhaseDone(phase) => {
                cli_sink::emit("phase", &format!("{phase:?}"));
                events.record("phase", serde_json::json!({"to": format!("{phase:?}")}))?;
            },
            RoundOutcome::Refused => {
                cli_sink::emit("gate", "decision refused; see the failure ledger");
            },
            RoundOutcome::Finished(reason) => {
                stop_reason = Some(reason.clone());
                break;
            },
        }

        // Breaker: stalled rounds trip the run.
        if progress.done_tasks > progress_before.done_tasks || productive_replans > 0 {
            // A completed task or a persisted plan body is productive
            // orchestrator work (v3r3: honest replans tripped the
            // stagnation breaker).
            breaker_state.record_productive_round();
            breaker_state.record_progress();
            round_ledger_observation(&run_dir, false);
        } else {
            breaker_state.record_stalled_round();
            round_ledger_observation(&run_dir, true);
        }
        let state = blackboard::state::read(&run_dir)?.expect("state present");
        match breaker_state.check(state.phase, &config.mas) {
            orchestrator::Breaker::Continue => {},
            orchestrator::Breaker::Trip(reason) => {
                stop_reason = Some(reason);
                break;
            },
        }
    }

    // Final state and result.
    let final_state = blackboard::state::read(&run_dir)?.expect("state present");
    let progress = orchestrator::progress::snapshot(&run_dir)?;
    let aggregate = experiment::Aggregate {
        harness_id: manifest.harness_id.clone(),
        final_phase: format!("{:?}", final_state.phase),
        completed: final_state.phase == Phase::Done,
        rounds: orchestrator_loop.round,
        delegations,
        tasks_done: progress.done_tasks,
        tasks_total: progress.total_tasks,
        stop_reason,
        duration_seconds: started.elapsed().as_secs(),
    };
    let result_target = invocation.experiment_dir.clone().unwrap_or_else(|| output_dir.clone());
    experiment::write_result(&result_target, &aggregate)?;
    events.record("done", serde_json::json!({"aggregate": &aggregate}))?;
    cli_sink::emit(
        "done",
        &format!(
            "phase={:?} rounds={} delegations={} tasks={}/{}",
            aggregate.final_phase, aggregate.rounds, aggregate.delegations, aggregate.tasks_done, aggregate.tasks_total
        ),
    );
    Ok(())
}

/// Record a round-progress observation in the ledger. `stalled` selects the
/// stalled-round kind instead of the productive-round one.
fn round_ledger_observation(run_dir: &Path, stalled: bool) {
    let ledger = Ledger::new(run_dir.join("ledgers"));
    let _ = ledger.append_observation(&blackboard::Observation {
        at: now_rfc3339(),
        kind: if stalled {
            "round_stalled"
        } else {
            "round_progress"
        }
        .into(),
        detail: serde_json::json!({}),
    });
}

/// Compact timestamp for ids: YYYYMMDDHHMMSS.
fn now_compact() -> String {
    time::OffsetDateTime::now_utc()
        .format(time::macros::format_description!("[year][month][day][hour][minute][second]"))
        .expect("fixed-format timestamp")
}

/// RFC 3339 UTC now.
fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339).expect("rfc3339 timestamp")
}
