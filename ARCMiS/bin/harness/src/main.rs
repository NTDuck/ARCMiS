//! Binary entry point for ARCMiS. The MAS harness driver: load config,
//! snapshot the source, write the manifest, run the manager loop under the
//! phase state machine, and emit the result aggregate.

mod cli_sink;
mod experiment;
mod observability;
mod offload;

use agents::util::provider::Provider;
use blackboard::Budgets;
use agents::Fleet;
use blackboard::Ledger;
use blackboard::Manifest;
use blackboard::Phase;
use blackboard::State;
use blackboard::TaskList;
use blackboard::Workspace;
use orchestrator::manager::ManagerLoop;
use orchestrator::manager::RoundOutcome;
use orchestrator::state_machine;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("harness failed: {error:#}");
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
                let id = args
                    .get(index + 1)
                    .ok_or_else(|| anyhow::anyhow!("--experiment needs an id"))?;
                experiment_dir = Some(PathBuf::from(format!(".artifacts/experiments/{id}")));
                index += 2;
            },
            "--config" => {
                config_path = PathBuf::from(
                    args.get(index + 1)
                        .ok_or_else(|| anyhow::anyhow!("--config needs a path"))?,
                );
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
            manager_turns: config.mas.manager_turns,
            worker_turns: config.mas.worker_turns,
            max_rounds: config.mas.max_rounds,
        },
        config_path: invocation.config_path.display().to_string(),
        problem_set: source_root
            .parent()
            .map(|parent| parent.display().to_string())
            .unwrap_or_default(),
        source_language: config.source.language.clone(),
        target_language: config.source.target.language.clone(),
        git_revision: String::new(),
        parents: Vec::new(),
        hypothesis: String::new(),
        created_at: now_rfc3339(),
    };
    blackboard::manifest::write(&run_dir, &manifest)?;
    // The workspace copy is the specialists' view of the run contract:
    // target language and toolchain come from here, not from guessing.
    let workspace_meta = output_dir.join("workspace").join("meta");
    std::fs::create_dir_all(&workspace_meta)?;
    std::fs::write(
        workspace_meta.join("run.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    if let Some(dir) = &invocation.experiment_dir {
        experiment::write_pre_run(dir, &manifest)?;
    }

    // Fleet: the ladder from the config; the manager on the top rung,
    // specialists on the weakest (the fleet analyst promotes).
    let ladder = if config.mas.model_ladder.is_empty() {
        vec![config.run.model.clone()]
    } else {
        config.mas.model_ladder.clone()
    };
    let manager_model = ladder.last().cloned().unwrap_or_else(|| config.run.model.clone());
    let mut fleet = Fleet {
        manager_model,
        ladder: ladder.clone(),
        role_models: Default::default(),
    };
    for role in agents::Role::ALL {
        if role != agents::Role::Manager {
            fleet.assign(role, ladder.first().cloned().unwrap_or_else(|| config.run.model.clone()));
        }
    }

    // Build the agents on the selected client, with each role's tool
    // allowlist rooted at the workspace. Tools resolve relative paths
    // against this root; prompts direct specialists at source/ and target/.
    let workspace_root = output_dir.join("workspace");
    let agents_set = match &clients {
        agents::util::provider::Clients::Ollama(client) => {
            agents::mas::registry::build(client, &fleet, &config.run, |role| {
                tools::build_tools(&workspace_root, role.allowed_tools())
            })?
        },
        agents::util::provider::Clients::Netmind(client) => {
            agents::mas::registry::build(client, &fleet, &config.run, |role| {
                tools::build_tools(&workspace_root, role.allowed_tools())
            })?
        },
    };

    // Initial state + task list.
    let state = State {
        phase: Phase::Preflight,
        phase_delegations: 0,
        current_task: None,
        current_batch: None,
        current_model: fleet.model_for(agents::Role::Manager).to_owned(),
        updated_at: now_rfc3339(),
        last_transition: "preflight".into(),
    };
    blackboard::state::write(&run_dir, &state)?;
    TaskList::new(&run_dir).write(&[])?;

    // Snapcompact offload hook: compact when the projected input passes half
    // the context window.
    let frame_budget = u32::try_from(config.run.num_ctx / 2).unwrap_or(u32::MAX);
    let _offload_hook = offload::hook(frame_budget);

    // The run loop.
    let events = observability::EventLog::new(&output_dir)?;
    let mut manager_loop = ManagerLoop {
        run_dir: run_dir.clone(),
        agents: agents_set,
        config: config.mas.clone(),
        workspace,
        ledger: Ledger::new(run_dir.join("ledgers")),
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
        let outcome = manager_loop.round().await?;
        let progress = orchestrator::progress::snapshot(&run_dir)?;

        match &outcome {
            RoundOutcome::Delegated { role, task, .. } => {
                delegations += 1;
                cli_sink::emit("delegate", &format!("{} <- {}", role.name(), task));
                events.record(
                    "delegation",
                    serde_json::json!({"role": role.name(), "task": task}),
                )?;
            },
            RoundOutcome::Replanned => cli_sink::emit("replan", "manager rewrote the plan"),
            RoundOutcome::Escalated(reason) => {
                cli_sink::emit("escalate", reason);
                events.record("escalate", serde_json::json!({"reason": reason}))?;
            },
            RoundOutcome::PhaseDone(phase) => {
                cli_sink::emit("phase", &format!("{phase:?}"));
                events.record("phase", serde_json::json!({"to": format!("{phase:?}")}))?;
            },
            RoundOutcome::Finished(reason) => {
                stop_reason = Some(reason.clone());
                break;
            },
        }

        // Breaker: stalled rounds trip the run.
        if progress.done_tasks > progress_before.done_tasks {
            breaker_state.record_productive_round();
            breaker_state.record_progress();
            manager_loop_ledger_progress(&run_dir);
        } else {
            breaker_state.record_stalled_round();
            manager_loop_ledger_stall(&run_dir);
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
        rounds: manager_loop.round,
        delegations,
        tasks_done: progress.done_tasks,
        tasks_total: progress.total_tasks,
        stop_reason,
        duration_seconds: u64::try_from(started.elapsed().as_secs()).unwrap_or(u64::MAX),
    };
    let result_target = invocation
        .experiment_dir
        .clone()
        .unwrap_or_else(|| output_dir.clone());
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

/// Record productive-round progress in the ledger.
fn manager_loop_ledger_progress(run_dir: &std::path::Path) {
    let ledger = Ledger::new(run_dir.join("ledgers"));
    let _ = ledger.append_observation(&blackboard::Observation {
        at: now_rfc3339(),
        kind: "round_progress".into(),
        detail: serde_json::json!({}),
    });
}

/// Record a stalled round in the ledger.
fn manager_loop_ledger_stall(run_dir: &std::path::Path) {
    let ledger = Ledger::new(run_dir.join("ledgers"));
    let _ = ledger.append_observation(&blackboard::Observation {
        at: now_rfc3339(),
        kind: "round_stalled".into(),
        detail: serde_json::json!({}),
    });
}

/// Compact timestamp for ids: YYYYMMDDHHMMSS.
fn now_compact() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let days = i64::try_from(seconds / 86_400).unwrap_or(0);
    let time = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}{month:02}{day:02}{h:02}{m:02}{s:02}",
        h = time / 3600,
        m = (time % 3600) / 60,
        s = time % 60
    )
}

/// RFC 3339 UTC now.
fn now_rfc3339() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let days = i64::try_from(seconds / 86_400).unwrap_or(0);
    let time = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time / 3600,
        (time % 3600) / 60,
        time % 60
    )
}

/// Days-since-epoch to civil date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
