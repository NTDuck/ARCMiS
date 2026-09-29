//! Task-graph executor: the dynamic-selection half of the orchestration
//! pattern (ADR 0022). The orchestrator's round may emit several `delegate`
//! decisions; each becomes a task with optional dependencies. Ready tasks
//! (all dependencies done) run bounded by the configured fan-out — with a
//! single ollama slot the interleaving is turn-level, not token-level.
//! A `team` delegation runs the translation collective (translator →
//! validator → tester, repairer re-entry) as a nested loop under one task.

use std::path::Path;

use agents::util::config::MasConfig;
use agents::MasAgents;
use agents::Role;
use blackboard::Ledger;
use blackboard::TaskList;
use blackboard::TaskStatus;
use blackboard::Workspace;

use crate::guard_hook::GuardHook;
use crate::loop_::judge_output;
use crate::loop_::now_string;
use crate::loop_::prompt_with_retries;
use crate::loop_::TurnOutput;

/// One parsed `delegate` decision.
#[derive(Debug, Clone)]
pub struct Delegation {
    /// Role hint from the orchestrator (empty → router picks).
    pub role: Option<String>,
    /// Task instruction.
    pub task: String,
    /// Task ids this delegation waits on.
    pub depends_on: Vec<String>,
    /// Run the translation collective under this task.
    pub team: bool,
}

/// Parse `after:t2,t3` and `team` clauses from the tail of a delegate body.
#[must_use]
pub fn parse_delegation(role: Option<String>, task: String) -> Delegation {
    let mut delegation = Delegation {
        role,
        task: String::new(),
        depends_on: Vec::new(),
        team: false,
    };
    // Split trailing `| clause` segments. A segment is a clause only when it
    // matches the grammar (`team` or `after:<ids>`); anything else is task
    // text that happens to contain a pipe and stays in the instruction.
    let segments = task.split('|').map(str::trim);
    let mut task_parts: Vec<String> = Vec::new();
    for segment in segments {
        if segment == "team" {
            delegation.team = true;
        } else if let Some(list) = segment.strip_prefix("after:") {
            delegation.depends_on =
                list.split(',').map(str::trim).filter(|id| !id.is_empty()).map(str::to_owned).collect();
        } else {
            task_parts.push(segment.to_owned());
        }
    }
    delegation.task = task_parts.join("|");
    delegation
}

/// Outcome of running the round's delegation set.
#[derive(Debug, Clone)]
pub struct BatchOutcome {
    /// Per-task results in execution order.
    pub results: Vec<TaskResult>,
    /// Registrations that stayed queued behind unfinished dependencies.
    pub queued: usize,
}

/// One task's execution result.
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Task id in the ledger.
    pub id: String,
    /// Role that ran it.
    pub role: Role,
    /// Whether the judge passed it.
    pub passed: bool,
    /// Specialist output (or the failure text).
    pub output: String,
}

/// Register the round's delegations in the task list and run the ready set.
pub async fn execute(
    agents: &MasAgents,
    config: &MasConfig,
    workspace: &Workspace,
    ledger: &Ledger,
    run_dir: &Path,
    phase: blackboard::Phase,
    delegations: Vec<Delegation>,
) -> anyhow::Result<BatchOutcome> {
    let tasks = TaskList::new(run_dir);
    let existing = tasks.read()?;
    // Re-delegation dedupe: (id, task text, deps) of tasks still queued. A
    // re-delegation of a Pending task reuses its id instead of forking a
    // duplicate entry; a Done or Blocked task earns a fresh id, because the
    // retry is a new, judged attempt.
    let mut queued: Vec<(String, String, Vec<String>)> = existing
        .iter()
        .filter(|task| task.status == TaskStatus::Pending)
        .map(|task| (task.id.clone(), task.description.clone(), task.depends_on.clone()))
        .collect();
    let mut registered: Vec<(String, Delegation)> = Vec::new();
    for delegation in delegations {
        let id = match queued
            .iter()
            .find(|(_, task, depends_on)| *task == delegation.task && *depends_on == delegation.depends_on)
        {
            Some((id, ..)) => id.clone(),
            None => {
                let id = tasks.add(&delegation.task, &delegation.depends_on)?;
                queued.push((id.clone(), delegation.task.clone(), delegation.depends_on.clone()));
                id
            },
        };
        registered.push((id, delegation));
    }

    // Ready set: dependencies done. An unknown dependency id reads as
    // satisfied so a mistyped `after:` cannot silently stall the task; the
    // orchestrator sees the task still pending and can re-delegate it.
    let known = tasks.read()?;
    let done: Vec<String> =
        known.iter().filter(|task| task.status == TaskStatus::Done).map(|task| task.id.clone()).collect();
    let registered_count = registered.len();
    let mut ready: Vec<(String, Delegation)> = registered
        .into_iter()
        .filter(|(_, delegation)| {
            delegation.depends_on.iter().all(|dep| done.contains(dep) || !known.iter().any(|task| &task.id == dep))
        })
        .collect();
    ready.truncate(config.fanout.max(1));
    for (id, _) in &ready {
        tasks.set_status(id, TaskStatus::InProgress)?;
    }

    let mut results = Vec::new();
    for (id, delegation) in ready {
        let result = if delegation.team {
            run_collective(agents, config, workspace, ledger, run_dir, phase, &id, &delegation).await
        } else {
            run_single(agents, config, workspace, ledger, phase, &id, &delegation).await
        };
        if result.passed {
            tasks.set_status(&result.id, TaskStatus::Done)?;
        } else {
            tasks.set_status(&result.id, TaskStatus::Blocked)?;
        }
        results.push(result);
    }
    Ok(BatchOutcome {
        queued: registered_count,
        results,
    })
}

/// Route a delegation to its role: orchestrator hint, then keyword pass, then
/// the default. The router model is skipped. The orchestrator's next round sees
/// an unhandled task and names a role; routing twice per task spends context
/// for nothing.
fn resolve_role(delegation: &Delegation, task_text: &str) -> Role {
    delegation
        .role
        .as_deref()
        .and_then(Role::from_name)
        .or_else(|| crate::router::route_by_keywords(task_text))
        .unwrap_or(Role::Translator)
}

/// Run one delegation: guard, specialist, judge, ledger.
async fn run_single(
    agents: &MasAgents,
    config: &MasConfig,
    workspace: &Workspace,
    ledger: &Ledger,
    phase: blackboard::Phase,
    task_id: &str,
    delegation: &Delegation,
) -> TaskResult {
    let role = resolve_role(delegation, &delegation.task);
    // Roles with a promised or verifiable-by-construction stop condition get
    // an enforced tool budget: a specialist that loops verification bash
    // otherwise burns the full turn budget and dies without a report.
    let mut policy = config.guard.clone();
    match role {
        Role::Validator | Role::Critic => policy.max_tool_calls = 20,
        // The tester's job is bounded (build, test, one sim run); 40 calls
        // absorb retries without letting re-verification loops eat the
        // 40-turn delegation budget (observed: 84 calls after a PASS).
        Role::Tester => policy.max_tool_calls = 40,
        _ => {},
    }
    let guard = GuardHook::new(role, workspace.clone(), policy, config.num_ctx);
    ledger
        .append_decision(&blackboard::Decision {
            at: now_string(),
            phase: format!("{phase:?}"),
            action: "delegate".into(),
            detail: serde_json::json!({"role": role.name(), "task": delegation.task, "id": task_id}),
            reasoning: "task-graph dispatch".into(),
        })
        .ok();

    let Some(agent) = agents.agent(role) else {
        return TaskResult {
            id: task_id.to_owned(),
            role,
            passed: false,
            output: format!("no agent for role {}", role.name()),
        };
    };
    let mut instruction = format!(
        "{}\n\nRun phase: {:?}. Guard: edit only inside target/ (tool paths resolve from the workspace root; meta/ is \
         the blackboard). When the deliverable is written, append a 3-5 line summary of what you did and where the \
         deliverable lives to meta/notes.md (create it if missing), then stop. Do not re-read your own output.",
        delegation.task, phase
    );
    if matches!(role, Role::Validator | Role::Critic) {
        instruction.push_str("\n\nThe translated workspace lives under target/; start from target/Cargo.toml.");
    }
    let turns = match role {
        Role::Validator | Role::Critic | Role::FailureAnalyst | Role::FleetAnalyst => config.judge_turns,
        _ => config.worker_turns,
    };
    let turn =
        match prompt_with_retries(agent, &instruction, config.max_repairs.max(1) as u32, Some(guard), turns).await {
            Ok(turn) => turn,
            Err(error) => TurnOutput {
                text: format!("DELEGATION FAILED: {error}"),
                tool_calls: Vec::new(),
            },
        };
    let output = turn.judgeable();
    finish_task(ledger, task_id, role, &output, &turn.tool_calls)
}

/// Run the translation collective under one task id: translator → validator,
/// with failure-analyst → repairer re-entry while repairs remain. The
/// collective stops early on a validator pass.
#[allow(clippy::too_many_arguments)]
async fn run_collective(
    agents: &MasAgents,
    config: &MasConfig,
    workspace: &Workspace,
    ledger: &Ledger,
    run_dir: &Path,
    phase: blackboard::Phase,
    task_id: &str,
    delegation: &Delegation,
) -> TaskResult {
    let mut repairs_left = config.max_repairs;
    let mut last_output;

    let translate = Delegation {
        role: Some("translator".into()),
        task: delegation.task.clone(),
        depends_on: Vec::new(),
        team: false,
    };
    let single = run_single(agents, config, workspace, ledger, phase, task_id, &translate).await;
    if !single.passed {
        return single;
    }
    last_output = single.output.clone();

    loop {
        let validate = Delegation {
            role: Some("validator".into()),
            task: format!(
                "Validate the translated batch produced under task {task_id}: run the target toolchain checks and the \
                 test command on target/."
            ),
            depends_on: Vec::new(),
            team: false,
        };
        let verdict = run_single(agents, config, workspace, ledger, phase, task_id, &validate).await;
        if verdict.passed {
            return TaskResult {
                id: task_id.to_owned(),
                role: Role::Translator,
                passed: true,
                output: last_output,
            };
        }
        if repairs_left == 0 {
            return TaskResult {
                id: task_id.to_owned(),
                role: Role::Translator,
                passed: false,
                output: format!("collective exhausted repairs: {}", verdict.output),
            };
        }
        repairs_left -= 1;
        let diagnose = Delegation {
            role: Some("failure-analyst".into()),
            task: format!("Classify this validation failure and give one suggested action: {}", verdict.output),
            depends_on: Vec::new(),
            team: false,
        };
        let diagnosis = run_single(agents, config, workspace, ledger, phase, task_id, &diagnose).await;
        let repair = Delegation {
            role: Some("repairer".into()),
            task: format!("Apply this repair to the target workspace: {}", diagnosis.output),
            depends_on: Vec::new(),
            team: false,
        };
        let repaired = run_single(agents, config, workspace, ledger, phase, task_id, &repair).await;
        last_output = format!("{}\n{}", last_output, repaired.output);
        if !repaired.passed {
            let tasks = TaskList::new(run_dir);
            tasks.set_status(task_id, TaskStatus::Blocked).ok();
            return TaskResult {
                id: task_id.to_owned(),
                role: Role::Translator,
                passed: false,
                output: format!("repair failed: {}", repaired.output),
            };
        }
    }
}

/// Failure root-cause detail: name the tool calls on a tool-only turn, else
/// the output head.
fn failure_detail(output: &str, tool_calls: &[String]) -> String {
    if output.trim().is_empty() && !tool_calls.is_empty() {
        return format!("[{} tool calls: {}]", tool_calls.len(), tool_calls.join(", "));
    }
    output.chars().take(200).collect()
}

/// Write the verdict to the ledger. `tool_calls` are the turn's executed tool
/// calls: a judged failure names them in the root cause so a tool-only turn
/// is not triaged as a silent model failure.
fn finish_task(ledger: &Ledger, task_id: &str, role: Role, output: &str, tool_calls: &[String]) -> TaskResult {
    let verdict = judge_output(role, output);
    let passed = verdict.is_ok();
    if passed {
        ledger
            .append_observation(&blackboard::Observation {
                at: now_string(),
                kind: "delegation_pass".into(),
                detail: serde_json::json!({"role": role.name(), "task": task_id}),
            })
            .ok();
    } else {
        let reason = verdict.err().unwrap_or_default();
        ledger
            .append_failure(&blackboard::Failure {
                at: now_string(),
                phase: String::new(),
                category: "model".into(),
                root_cause: format!("{reason}: {}", failure_detail(output, tool_calls)),
                suggested_action: "re-delegate with a tighter instruction".into(),
            })
            .ok();
    }
    TaskResult {
        id: task_id.to_owned(),
        role,
        passed,
        output: output.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_after_and_team_clauses() {
        let delegation = parse_delegation(Some("translator".into()), "Port batch 1 | after: t2, t3 | team".into());
        assert_eq!(delegation.task, "Port batch 1");
        assert_eq!(delegation.depends_on, vec!["t2", "t3"]);
        assert!(delegation.team);
    }

    #[test]
    fn plain_task_has_no_clauses() {
        let delegation = parse_delegation(None, "Map the source modules".into());
        assert_eq!(delegation.task, "Map the source modules");
        assert!(delegation.depends_on.is_empty());
        assert!(!delegation.team);
    }

    #[test]
    fn task_text_may_contain_pipes_in_first_segment_only() {
        // The first `|` ends the task text when a clause follows; without
        // clauses the whole body is the task.
        let delegation = parse_delegation(Some("critic".into()), "Review a|b".into());
        assert_eq!(delegation.task, "Review a|b");
    }
}
