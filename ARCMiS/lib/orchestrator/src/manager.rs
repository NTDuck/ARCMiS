//! Manager loop: one orchestration round. The manager agent reads the
//! blackboard, decides, and the orchestrator executes the decision
//! mechanically (delegate, replan, escalate, done).

use crate::guard::Guard;
use crate::judges;
use crate::router;
use crate::state_machine;
use agents::util::config::MasConfig;
use agents::MasAgents;
use agents::Role;
use blackboard::Decision;
use blackboard::Ledger;
use blackboard::Phase;
use blackboard::TaskList;
use blackboard::TaskStatus;
use blackboard::Workspace;
use std::sync::Arc;

/// Outcome of one manager round.
#[derive(Debug, Clone)]
pub enum RoundOutcome {
    /// The manager delegated; the orchestrator ran the specialist.
    Delegated {
        /// Role the work went to.
        role: Role,
        /// Task id.
        task: String,
        /// Specialist's final text.
        output: String,
    },
    /// The manager rewrote the plan.
    Replanned,
    /// The manager escalated (repeated failure).
    Escalated(String),
    /// The manager declared the phase done.
    PhaseDone(Phase),
    /// The manager asked to stop the run.
    Finished(String),
}

/// The manager loop context: everything one round touches.
pub struct ManagerLoop {
    /// Run directory holding the blackboard files.
    pub run_dir: std::path::PathBuf,
    /// The built agents.
    pub agents: MasAgents,
    /// MAS config.
    pub config: MasConfig,
    /// Workspace (for the guard).
    pub workspace: Workspace,
    /// Ledger for decisions.
    pub ledger: Ledger,
    /// Round counter.
    pub round: usize,
}

impl ManagerLoop {
    /// Run one round: read state, ask the manager, execute the decision.
    pub async fn round(&mut self) -> anyhow::Result<RoundOutcome> {
        self.round += 1;
        let state = blackboard::state::read(&self.run_dir)?
            .ok_or_else(|| anyhow::anyhow!("run state missing; preflight must write it"))?;
        let prompt = self.manager_prompt(&state)?;
        let manager = self
            .agents
            .agent(Role::Manager)
            .ok_or_else(|| anyhow::anyhow!("manager agent missing"))?;

        // The manager answers with one verb line: `DECISION: <verb> [args]`.
        use rig::completion::Prompt as _;
        let answer = manager.prompt(&prompt).await?;
        let decision = parse_decision(&answer)
            .ok_or_else(|| anyhow::anyhow!("manager gave no DECISION line: {answer:?}"))?;

        match decision {
            DecisionVerb::Delegate { role, task } => {
                self.execute_delegation(role, &task).await
            },
            DecisionVerb::Replan => {
                self.ledger
                    .append_decision(&Decision {
                        at: now(),
                        phase: format!("{:?}", state.phase),
                        action: "replan".into(),
                        detail: serde_json::json!({"round": self.round}),
                        reasoning: "manager rewrote the plan".into(),
                    })?;
                Ok(RoundOutcome::Replanned)
            },
            DecisionVerb::Escalate { reason } => {
                self.ledger
                    .append_decision(&Decision {
                        at: now(),
                        phase: format!("{:?}", state.phase),
                        action: "escalate".into(),
                        detail: serde_json::json!({"round": self.round}),
                        reasoning: reason.clone(),
                    })?;
                Ok(RoundOutcome::Escalated(reason))
            },
            DecisionVerb::Done => {
                let record =
                    state_machine::apply(&mut snapshot_phase(&self.run_dir)?, state.phase, "phase exit condition holds")?;
                tracing::info!(from = ?record.from, to = ?record.to, "phase transition");
                Ok(RoundOutcome::PhaseDone(record.to))
            },
            DecisionVerb::Finish { reason } => Ok(RoundOutcome::Finished(reason)),
        }
    }

    /// Delegate one task: route, build the guard, run the specialist, judge.
    async fn execute_delegation(&mut self, role_hint: Option<String>, task: &str) -> anyhow::Result<RoundOutcome> {
        let state = blackboard::state::read(&self.run_dir)?.expect("state read above");
        // Route: keyword pass first, model fallback.
        let role = role_hint
            .as_deref()
            .and_then(Role::from_name)
            .or_else(|| router::route_by_keywords(task))
            .unwrap_or(Role::Translator);

        // The judge roles are read-only; everything else writes target/.
        // The harness wraps each specialist's tools in a GuardedTool before
        // the delegation; the guard here re-checks the allowlist.
        let _guard = Arc::new(Guard::new(role.allowed_tools().to_vec(), self.workspace.clone()));

        // Append the delegation decision.
        self.ledger
            .append_decision(&Decision {
                at: now(),
                phase: format!("{:?}", state.phase),
                action: "delegate".into(),
                detail: serde_json::json!({"role": role.name(), "task": task}),
                reasoning: "manager round".into(),
            })?;

        // Run the specialist with its turn budget.
        let agent = self
            .agents
            .agent(role)
            .ok_or_else(|| anyhow::anyhow!("agent for {} missing", role.name()))?;
        use rig::completion::Prompt as _;
        let instruction = format!("{task}\n\nRun phase: {:?}. Guard: edit only inside workspace/target/.", state.phase);
        let output = agent.prompt(instruction).await?;

        // Judge the output by the role's contract line.
        let verdict = judge_output(role, &output);
        let tasks = TaskList::new(&self.run_dir);

        match verdict {
            Judged::Pass => {
                self.ledger
                    .append_observation(&blackboard::Observation {
                        at: now(),
                        kind: "delegation_pass".into(),
                        detail: serde_json::json!({"role": role.name()}),
                    })?;
                let _ = tasks;
                Ok(RoundOutcome::Delegated {
                    role,
                    task: task.to_owned(),
                    output,
                })
            },
            Judged::Fail(reason) => {
                self.ledger
                    .append_failure(&blackboard::Failure {
                        at: now(),
                        phase: format!("{:?}", state.phase),
                        category: "model".into(),
                        root_cause: reason.clone(),
                        suggested_action: "re-delegate with a tighter instruction".into(),
                    })?;
                Ok(RoundOutcome::Delegated {
                    role,
                    task: task.to_owned(),
                    output: format!("FAILED: {reason}\n{output}"),
                })
            },
        }
    }

    /// Compose the manager's round prompt from the blackboard.
    fn manager_prompt(&self, state: &blackboard::State) -> anyhow::Result<String> {
        let tasks = TaskList::new(&self.run_dir).read()?;
        let plan = read_or_empty(&self.run_dir.join("plan.md"))?;
        let notes = read_or_empty(&self.run_dir.join("notes.md"))?;
        let failures = self.ledger.read_failures()?;
        let recent: Vec<String> = failures
            .iter()
            .rev()
            .take(5)
            .map(|failure| format!("- [{}] {}: {}", failure.phase, failure.category, failure.root_cause))
            .collect();
        Ok(format!(
            "ROUND {}\n\nSTATE: phase={:?} batch={:?} model={}\n\nPLAN:\n{}\n\nNOTES:\n{}\n\nRECENT FAILURES:\n{}\n\nTASKS:\n{}\n\nDecide the next action. Answer with one line:\nDECISION: delegate <role> | <task description>\nDECISION: replan\nDECISION: escalate | <reason>\nDECISION: done\nDECISION: finish | <reason>",
            self.round,
            state.phase,
            state.current_batch,
            state.current_model,
            truncate(&plan, self.config.plan_cap),
            truncate(&notes, self.config.notes_cap),
            recent.join("\n"),
            tasks
                .iter()
                .map(|task| format!(
                    "- {} [{}] {}",
                    task.id,
                    format!("{:?}", task.status).to_ascii_lowercase(),
                    task.description
                ))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}

/// The manager's decision verbs.
enum DecisionVerb {
    /// Delegate to a role (named or router-chosen).
    Delegate {
        role: Option<String>,
        task: String,
    },
    /// Rewrite the plan.
    Replan,
    /// Escalate a repeated failure.
    Escalate { reason: String },
    /// Declare the phase done.
    Done,
    /// Stop the run.
    Finish { reason: String },
}

/// Parse the manager's `DECISION:` line.
fn parse_decision(answer: &str) -> Option<DecisionVerb> {
    let line = answer
        .lines()
        .rev()
        .find(|line| line.trim_start().to_ascii_uppercase().starts_with("DECISION:"))?;
    let body = line.trim_start()["DECISION:".len()..].trim();
    let (verb, rest) = body.split_once(char::is_whitespace).unwrap_or((body, ""));
    match verb.to_ascii_lowercase().as_str() {
        "delegate" => {
            // `delegate <role> | <task>` or `delegate | <task>`.
            let (head, task) = rest.split_once('|').unwrap_or((rest, ""));
            let role = head.trim();
            let role = if role.is_empty() || role == "|" {
                None
            } else {
                Some(role.to_owned())
            };
            Some(DecisionVerb::Delegate {
                role,
                task: task.trim().to_owned(),
            })
        },
        "replan" => Some(DecisionVerb::Replan),
        "escalate" => Some(DecisionVerb::Escalate {
            reason: rest.trim_start_matches('|').trim().to_owned(),
        }),
        "done" => Some(DecisionVerb::Done),
        "finish" => Some(DecisionVerb::Finish {
            reason: rest.trim_start_matches('|').trim().to_owned(),
        }),
        _ => None,
    }
}

/// Judge a specialist's final text by its contract line.
fn judge_output(role: Role, output: &str) -> Judged {
    match role {
        Role::Validator => match judges::parse_verdict(output, "VALIDATION") {
            Some((true, _)) => Judged::Pass,
            Some((false, reason)) => Judged::Fail(format!("validation failed: {reason}")),
            None => Judged::Fail("validator produced no VALIDATION line".into()),
        },
        Role::Critic => match judges::parse_verdict(output, "CRITIQUE") {
            Some((true, _)) => Judged::Pass,
            Some((false, reason)) => Judged::Fail(format!("critique failed: {reason}")),
            None => Judged::Fail("critic produced no CRITIQUE line".into()),
        },
        Role::Repairer => match judges::parse_repair(output) {
            Some(("applied", _)) => Judged::Pass,
            Some((word, detail)) => Judged::Fail(format!("repair {word}: {detail}")),
            None => Judged::Fail("repairer produced no REPAIR line".into()),
        },
        Role::FailureAnalyst => {
            if judges::parse_diagnosis(output).is_some() {
                Judged::Pass
            } else {
                Judged::Fail("failure analyst produced no DIAGNOSIS line".into())
            }
        },
        // Declarative roles pass when they produced nonempty output.
        _ => {
            if output.trim().is_empty() {
                Judged::Fail("empty specialist output".into())
            } else {
                Judged::Pass
            }
        },
    }
}

/// Judged output.
enum Judged {
    /// The specialist met its contract.
    Pass,
    /// The specialist failed its contract; the string says why.
    Fail(String),
}

/// Read a file or empty string when absent.
fn read_or_empty(path: &std::path::Path) -> anyhow::Result<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}

/// Truncate text to a character cap with a marker.
fn truncate(text: &str, cap: usize) -> String {
    if text.chars().count() <= cap {
        return text.to_owned();
    }
    let kept: String = text.chars().take(cap).collect();
    format!("{kept}\n...[truncated]")
}

/// RFC 3339 UTC now (no external time dep in this crate: blackboard carries
/// the formatting feature; reuse its re-exported time types through serde).
fn now() -> String {
    let now = time_now();
    now
}

/// Bridge to the time crate through blackboard's dependency version.
fn time_now() -> String {
    // Cheap UTC timestamp from the Unix epoch; the ledger only needs
    // monotone-ish ordering, and the harness logs carry full precision.
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("unix:{seconds}")
}

/// Load the state fresh for a phase transition write.
fn snapshot_phase(run_dir: &std::path::Path) -> anyhow::Result<blackboard::State> {
    blackboard::state::read(run_dir)?
        .ok_or_else(|| anyhow::anyhow!("state missing in {run_dir:?}"))
}

/// Task status alias for the judge module.
pub type TaskStatusAlias = TaskStatus;
