//! Task-graph executor: the dynamic-selection half of the orchestration
//! pattern (ADR 0022). The orchestrator's round may emit several `delegate`
//! decisions. each becomes a task with optional dependencies. Ready tasks
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
    /// Tier-2 lead hint: the orchestrator may name a lead directly
    /// (hierarchical mode). Empty = route by role, then team lookup.
    pub lead: Option<String>,
}

/// Parse `after:t2,t3` and `team` clauses from the tail of a delegate body.
#[must_use]
pub(crate) fn parse_delegation(role: Option<String>, task: String) -> Delegation {
    let mut delegation = Delegation {
        role,
        task: String::new(),
        depends_on: Vec::new(),
        team: false,
        lead: None,
    };
    // Split trailing `| clause` segments. A segment is a clause only when it
    // matches the grammar (`team` or `after:<ids>`). anything else is task
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
    /// Delegating tier (1 = orchestrator, 2 = lead). ADR 0026.
    pub tier: u8,
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
    // duplicate entry. a Done or Blocked task earns a fresh id, because the
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
    // satisfied so a mistyped `after:` cannot silently stall the task. the
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
        let result = dispatch(agents, config, workspace, ledger, run_dir, phase, &id, &delegation).await;
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

/// Run one ready task: single-tier dispatch (ADR 0022) or the hierarchical
/// handoff (ADR 0026). With hierarchy off this is byte-identical to the
/// old direct call. With hierarchy on a specialist-named batch delegation
/// routes to the member's lead (or is refused under `deny_direct`), and a
/// lead-named delegation runs the lead's bounded inner loop.
async fn dispatch(
    agents: &MasAgents,
    config: &MasConfig,
    workspace: &Workspace,
    ledger: &Ledger,
    run_dir: &Path,
    phase: blackboard::Phase,
    task_id: &str,
    delegation: &Delegation,
) -> TaskResult {
    let teams = crate::hierarchy::teams(config);
    match route_tier1(delegation, &teams, config, &|name: &str| agents.lead(name).is_some()) {
        TierRoute::Refuse {
            role,
            lead,
        } => {
            ledger
                .append_failure(&blackboard::Failure {
                    at: now_string(),
                    phase: format!("{phase:?}"),
                    category: "gate".into(),
                    root_cause: format!(
                        "tier-1 delegation names specialist '{}' while hierarchy is on; delegate to lead '{lead}' \
                         instead",
                        role.name()
                    ),
                    suggested_action: format!("delegate {lead} with this task"),
                })
                .ok();
            TaskResult {
                id: task_id.to_owned(),
                role,
                passed: false,
                output: "refused: tier-1 must delegate batch work to a team lead".into(),
                tier: 1,
            }
        },
        TierRoute::Lead(lead_name) =>
            crate::lead::run_lead_batch(agents, config, workspace, ledger, phase, task_id, &lead_name, delegation).await,
        TierRoute::Direct =>
            if delegation.team {
                run_collective(agents, config, workspace, ledger, run_dir, phase, task_id, delegation).await
            } else {
                run_single(agents, config, workspace, ledger, phase, task_id, delegation, 1).await
            },
    }
}

/// Tier-1 routing under hierarchy (ADR 0026). The orchestrator names a
/// lead -> handoff. It names a specialist -> the owning lead, unless
/// `deny_direct` refuses the direct-to-specialist shortcut. No team and no
/// lead agent for the name -> normal single dispatch (advisory roles like
/// the fleet analyst stay tier-1 accessible).
#[derive(Debug)]
enum TierRoute {
    /// Delegate to this lead's inner loop.
    Lead(String),
    /// Refuse with gate feedback (the orchestrator re-delegates).
    Refuse {
        role: Role,
        lead: String,
    },
    /// Execute directly at tier 1.
    Direct,
}

fn route_tier1(
    delegation: &Delegation,
    teams: &[agents::mas::leads::Team],
    config: &MasConfig,
    lead_exists: &dyn Fn(&str) -> bool,
) -> TierRoute {
    // Hierarchy off: byte-identical to pre-hierarchy behavior (acceptance
    // d, ADR 0026) — no team lookup, no lead agents, tier stays 1.
    if !config.hierarchy.enabled {
        return TierRoute::Direct;
    }
    if let Some(lead) = delegation.lead.clone() {
        return if lead_exists(&lead) {
            TierRoute::Lead(lead)
        } else {
            TierRoute::Direct
        };
    }
    // The round prompt lists lead names in ROLE when hierarchy is on, so
    // `delegate discovery-lead | ...` arrives as an unresolved role name.
    // A name that matches a built lead is the handoff; anything else that
    // fails to resolve keeps the old router fallback.
    if let Some(name) = delegation.role.as_deref() {
        if lead_exists(name) {
            return TierRoute::Lead(name.to_owned());
        }
    }
    let Some(role) = delegation.role.as_deref().and_then(Role::from_name) else {
        return TierRoute::Direct;
    };
    match crate::hierarchy::lead_for(teams, role) {
        Some(team) if lead_exists(&team.lead) =>
            if config.hierarchy.deny_direct {
                TierRoute::Refuse {
                    role,
                    lead: team.lead.clone(),
                }
            } else {
                TierRoute::Lead(team.lead.clone())
            },
        _ => TierRoute::Direct,
    }
}

/// Route a delegation to its role: orchestrator hint, then keyword pass, then
/// the default. The router model is skipped. The orchestrator's next round sees
/// an unhandled task and names a role. routing twice per task spends context
/// for nothing.
fn resolve_role(delegation: &Delegation, task_text: &str) -> Role {
    delegation
        .role
        .as_deref()
        .and_then(Role::from_name)
        .or_else(|| crate::router::route_by_keywords(task_text))
        .unwrap_or(Role::Translator)
}

/// Run one delegation: guard, specialist, judge, ledger. `tier` names the
/// delegating tier (1 = orchestrator, 2 = lead). it lands in the ledger
/// decision so the hierarchy stays observable outside traces (ADR 0026).
pub(crate) async fn run_single(
    agents: &MasAgents,
    config: &MasConfig,
    workspace: &Workspace,
    ledger: &Ledger,
    phase: blackboard::Phase,
    task_id: &str,
    delegation: &Delegation,
    tier: u8,
) -> TaskResult {
    let role = resolve_role(delegation, &delegation.task);
    // Roles with a promised or verifiable-by-construction stop condition get
    // an enforced tool budget: a specialist that loops verification bash
    // otherwise burns the full turn budget and dies without a report.
    let mut policy = config.guard.clone();
    match role {
        Role::Validator | Role::Critic => policy.max_tool_calls = 20,
        // The tester's job is bounded (build, test, one sim run). 40 calls
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
            detail: serde_json::json!({"role": role.name(), "task": delegation.task, "id": task_id, "tier": tier}),
            reasoning: "task-graph dispatch".into(),
        })
        .ok();

    let Some(agent) = agents.agent(role) else {
        return TaskResult {
            id: task_id.to_owned(),
            role,
            passed: false,
            output: format!("no agent for role {}", role.name()),
            tier,
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
    finish_task(ledger, task_id, role, &output, &turn.tool_calls, tier)
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
        lead: None,
    };
    let single = run_single(agents, config, workspace, ledger, phase, task_id, &translate, 1).await;
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
            lead: None,
        };
        let verdict = run_single(agents, config, workspace, ledger, phase, task_id, &validate, 1).await;
        if verdict.passed {
            return TaskResult {
                id: task_id.to_owned(),
                role: Role::Translator,
                passed: true,
                output: last_output,
                tier: 1,
            };
        }
        if repairs_left == 0 {
            return TaskResult {
                id: task_id.to_owned(),
                role: Role::Translator,
                passed: false,
                output: format!("collective exhausted repairs: {}", verdict.output),
                tier: 1,
            };
        }
        repairs_left -= 1;
        let diagnose = Delegation {
            role: Some("failure-analyst".into()),
            task: format!("Classify this validation failure and give one suggested action: {}", verdict.output),
            depends_on: Vec::new(),
            team: false,
            lead: None,
        };
        let diagnosis = run_single(agents, config, workspace, ledger, phase, task_id, &diagnose, 1).await;
        let repair = Delegation {
            role: Some("repairer".into()),
            task: format!("Apply this repair to the target workspace: {}", diagnosis.output),
            depends_on: Vec::new(),
            team: false,
            lead: None,
        };
        let repaired = run_single(agents, config, workspace, ledger, phase, task_id, &repair, 1).await;
        last_output = format!("{}\n{}", last_output, repaired.output);
        if !repaired.passed {
            let tasks = TaskList::new(run_dir);
            tasks.set_status(task_id, TaskStatus::Blocked).ok();
            return TaskResult {
                id: task_id.to_owned(),
                role: Role::Translator,
                passed: false,
                output: format!("repair failed: {}", repaired.output),
                tier: 1,
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
fn finish_task(
    ledger: &Ledger,
    task_id: &str,
    role: Role,
    output: &str,
    tool_calls: &[String],
    tier: u8,
) -> TaskResult {
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
        tier,
    }
}

#[cfg(test)]
mod tests {
    use agents::util::config::HierarchyConfig;

    use super::*;

    /// The lead-presence predicate the tests inject: one configured lead
    /// name. Routing checks names, not agent contents, so no client is
    /// needed.
    fn lead_exists(lead: &str) -> impl Fn(&str) -> bool + '_ {
        let configured = lead.to_owned();
        move |name: &str| name == configured
    }

    fn hierarchy_config() -> MasConfig {
        let mut config = MasConfig::default();
        config.hierarchy = HierarchyConfig {
            enabled: true,
            deny_direct: true,
            teams: vec![agents::util::config::TeamConfig {
                lead: "migration-lead".into(),
                members: vec!["translator".into(), "validator".into()],
                turns: 4,
                stagnation_rounds: None,
            }],
        };
        config
    }

    /// (a) Hierarchy on: a tier-1 specialist-named batch delegation never
    /// executes directly at tier 1. it routes to the member's lead (or is
    /// refused under `deny_direct`), never reaching the member role.
    #[test]
    fn hierarchy_on_routes_specialist_to_lead_never_to_member() {
        let config = hierarchy_config();
        let teams = crate::hierarchy::teams(&config);

        let delegation = Delegation {
            role: Some("translator".into()),
            task: "Port batch 1".into(),
            depends_on: Vec::new(),
            team: true,
            lead: None,
        };
        match route_tier1(&delegation, &teams, &config, &lead_exists("migration-lead")) {
            TierRoute::Refuse {
                lead,
                ..
            } => assert_eq!(lead, "migration-lead"),
            other => panic!("expected refusal or lead routing, got {other:?}"),
        }

        // Without deny_direct the same delegation reaches the lead loop.
        let mut config_routed = config.clone();
        config_routed.hierarchy.deny_direct = false;
        match route_tier1(&delegation, &teams, &config_routed, &lead_exists("migration-lead")) {
            TierRoute::Lead(lead) => assert_eq!(lead, "migration-lead"),
            other => panic!("expected lead routing, got {other:?}"),
        }
    }

    /// (a, deny path) A specialist with an owning lead but no built lead
    /// agent falls back to direct dispatch. a role with no team (advisory
    /// roles) stays tier-1 accessible.
    #[test]
    fn hierarchy_advisory_roles_stay_direct() {
        let config = hierarchy_config();
        let teams = crate::hierarchy::teams(&config);
        let delegation = Delegation {
            role: Some("fleet-analyst".into()),
            task: "Report model promotion".into(),
            depends_on: Vec::new(),
            team: false,
            lead: None,
        };
        assert!(matches!(route_tier1(&delegation, &teams, &config, &lead_exists("migration-lead")), TierRoute::Direct));
    }

    /// (c) Depth cap: a tier-1 delegation naming a lead with no agent, or a
    /// lead attempting to name another lead, is refused or falls back — the
    /// lead parser test covers the lead side. here the direct path is a
    /// plain dispatch, never a sub-lead.
    #[test]
    fn lead_named_without_agent_stays_direct_not_sublead() {
        let config = hierarchy_config();
        let teams = crate::hierarchy::teams(&config);
        let delegation = Delegation {
            role: None,
            task: "Port batch 1".into(),
            depends_on: Vec::new(),
            team: false,
            lead: Some("discovery-lead".into()),
        };
        assert!(matches!(route_tier1(&delegation, &teams, &config, &lead_exists("migration-lead")), TierRoute::Direct));
    }

    /// (d) Hierarchy off: routing is direct for everything, including a
    /// specialist-named team delegation.
    #[test]
    fn hierarchy_off_routes_direct() {
        let mut config = hierarchy_config();
        config.hierarchy.enabled = false;
        let teams = crate::hierarchy::teams(&config);
        let delegation = Delegation {
            role: Some("translator".into()),
            task: "Port batch 1".into(),
            depends_on: Vec::new(),
            team: true,
            lead: None,
        };
        assert!(matches!(route_tier1(&delegation, &teams, &config, &lead_exists("migration-lead")), TierRoute::Direct));
    }

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
        // The first `|` ends the task text when a clause follows. without
        // clauses the whole body is the task.
        let delegation = parse_delegation(Some("critic".into()), "Review a|b".into());
        assert_eq!(delegation.task, "Review a|b");
    }
}
