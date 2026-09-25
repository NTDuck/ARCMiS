//! Manager loop: one orchestration round. The manager agent reads the
//! blackboard, decides, and the orchestrator executes the decision
//! mechanically (delegate, replan, escalate, done).

use crate::judges;
use crate::state_machine;
use agents::util::config::MasConfig;
use agents::MasAgents;
use agents::Role;
use blackboard::Decision;
use blackboard::Ledger;
use blackboard::Phase;
use blackboard::TaskList;
use blackboard::Workspace;
use std::path::Path;
use std::path::PathBuf;

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
    /// The harness refused a decision (evidence gate); the manager sees the
    /// refusal in the next round's failure tail.
    Refused,
    /// The manager asked to stop the run.
    Finished(String),
}

/// The manager loop context: everything one round touches.
pub struct ManagerLoop {
    /// Run directory holding the blackboard files.
    pub run_dir: PathBuf,
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
    /// Retries per model call on transient provider errors.
    pub max_retries: u32,
}

impl ManagerLoop {
    /// Run one round: read state, ask the manager, execute the decision.
    pub async fn round(&mut self) -> anyhow::Result<RoundOutcome> {
        self.round += 1;
        let state = blackboard::state::read(&self.run_dir)?
            .ok_or_else(|| anyhow::anyhow!("run state missing; preflight must write it"))?;
        // Mirror the blackboard into workspace/meta so every role reads the
        // run state through its sandbox; run/ itself is outside the tool root.
        self.mirror_blackboard()?;
        let prompt = self.manager_prompt(&state)?;
        let manager = self.agents.agent(Role::Manager).ok_or_else(|| anyhow::anyhow!("manager agent missing"))?;

        // The manager answers with one verb line: `DECISION: <verb> [args]`.
        let answer = prompt_with_retries(manager, &prompt, self.max_retries, None, self.config.manager_turns).await?;
        let decisions = parse_decisions(&answer);
        if decisions.is_empty() {
            return Err(anyhow::anyhow!("manager gave no DECISION line: {answer:?}"));
        }

        // The dynamic layer: several delegate decisions register into the
        // task graph and the ready set runs under the fan-out cap. Non-
        // delegate verbs short-circuit as before (a round that escalates or
        // declares done runs nothing).
        let delegates: Vec<_> = decisions
            .iter()
            .filter_map(|decision| match decision {
                DecisionVerb::Delegate {
                    role,
                    task,
                } => Some(crate::taskgraph::parse_delegation(role.clone(), task.clone())),
                _ => None,
            })
            .collect();
        if !delegates.is_empty() {
            let outcome = crate::taskgraph::execute(
                &self.agents,
                &self.config,
                &self.workspace,
                &self.ledger,
                &self.run_dir,
                state.phase,
                delegates,
            )
            .await?;
            let mut state = snapshot_phase(&self.run_dir)?;
            for result in &outcome.results {
                if result.passed {
                    state.phase_delegations += 1;
                }
            }
            blackboard::state::write(&self.run_dir, &state)?;
            let summary: Vec<String> = outcome
                .results
                .iter()
                .map(|result| {
                    format!(
                        "{} [{}] {}",
                        result.id,
                        result.role.name(),
                        if result.passed {
                            "pass"
                        } else {
                            "fail"
                        }
                    )
                })
                .collect();
            if let Some(first) = outcome.results.first() {
                return Ok(RoundOutcome::Delegated {
                    role: first.role,
                    task: summary.join("; "),
                    output: outcome.results.iter().map(|result| result.output.clone()).collect::<Vec<_>>().join("\n\n"),
                });
            }
            // Every delegation queued behind its dependencies; the manager's
            // next round sees the pending tasks. Report replan, not a
            // delegation with a fabricated role.
            return Ok(RoundOutcome::Replanned);
        }

        // Single non-delegate decision (first one wins). `Delegate` cannot
        // appear here: a non-empty delegate set ran through the task-graph
        // executor above, so decisions[0] is a non-delegate verb.
        let decision = &decisions[0];
        match decision {
            // Unreachable: a non-empty delegate set ran through the
            // task-graph executor above, so decisions[0] is a non-delegate
            // verb. The arm only satisfies exhaustiveness.
            DecisionVerb::Delegate {
                ..
            } => Ok(RoundOutcome::Replanned),
            DecisionVerb::Replan => {
                self.ledger.append_decision(&Decision {
                    at: now_string(),
                    phase: format!("{:?}", state.phase),
                    action: "replan".into(),
                    detail: serde_json::json!({"round": self.round}),
                    reasoning: "manager rewrote the plan".into(),
                })?;
                Ok(RoundOutcome::Replanned)
            },
            DecisionVerb::Escalate {
                reason,
            } => {
                self.ledger.append_decision(&Decision {
                    at: now_string(),
                    phase: format!("{:?}", state.phase),
                    action: "escalate".into(),
                    detail: serde_json::json!({"round": self.round}),
                    reasoning: reason.clone(),
                })?;
                Ok(RoundOutcome::Escalated(reason.clone()))
            },
            DecisionVerb::Done => {
                // A phase advances on evidence: at least one completed
                // delegation since the phase began. A bare claim advances
                // nothing (observed: the walk Pilot -> Migration ->
                // Integration in 18 s with no validation work).
                if state.phase_delegations == 0 {
                    self.ledger.append_failure(&blackboard::Failure {
                        at: now_string(),
                        phase: format!("{:?}", state.phase),
                        category: "gate".into(),
                        root_cause: "done claimed with no completed delegation in this phase; advance needs at least one judged pass".into(),
                        suggested_action: format!("delegate the remaining {:?} work first", state.phase),
                    })?;
                    return Ok(RoundOutcome::Refused);
                }
                // Advance to the successor phase, not a self-transition:
                // only Pilot/Migration/Integration may self-transition.
                let next = state_machine::next(state.phase);
                let mut current = snapshot_phase(&self.run_dir)?;
                let record = state_machine::apply(&mut current, next, "phase exit condition holds")?;
                blackboard::state::write(&self.run_dir, &current)?;
                tracing::info!(from = ?record.from, to = ?record.to, "phase transition");
                Ok(RoundOutcome::PhaseDone(record.to))
            },
            DecisionVerb::Finish {
                reason,
            } => Ok(RoundOutcome::Finished(reason.clone())),
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
            "ROUND {}\n\nSTATE: phase={:?} batch={:?} model={}\n\nPLAN:\n{}\n\nNOTES:\n{}\n\nRECENT FAILURES:\n{}\n\nTASKS:\n{}\n\nDecide the next action. Answer with one or more DECISION lines (plain words, no angle brackets, no function-call syntax):\nDECISION: delegate ROLE | TASK TEXT\nDECISION: delegate ROLE | TASK TEXT | after:t2,t3\nDECISION: delegate ROLE | TASK TEXT | team\nDECISION: replan\nDECISION: escalate | REASON\nDECISION: done\nDECISION: finish | REASON\nROLE is one of: analyst, architect, planner, translator, validator, tester, failure-analyst, critic, repairer, fleet-analyst.\nYou MAY emit several delegate lines in one round when the tasks are independent; the harness runs up to {fanout} of them in parallel and queues the rest. `after:` lists task ids this one waits on; `team` runs the translation collective (translate, validate, repair) end to end under one task.",
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
                .join("\n"),
            fanout = self.config.fanout,
        ))
    }
}

/// The manager's decision verbs.
pub(crate) enum DecisionVerb {
    /// Delegate to a role (named or router-chosen).
    Delegate {
        role: Option<String>,
        task: String,
    },
    /// Rewrite the plan.
    Replan,
    /// Escalate a repeated failure.
    Escalate {
        reason: String,
    },
    /// Declare the phase done.
    Done,
    /// Stop the run.
    Finish {
        reason: String,
    },
}

/// Parse every `DECISION:` line; the manager may emit several delegates in
/// one round (the task-graph executor orders and gates them).
pub(crate) fn parse_decisions(answer: &str) -> Vec<DecisionVerb> {
    answer
        .lines()
        .filter(|line| line.trim_start().to_ascii_uppercase().starts_with("DECISION:"))
        .filter_map(parse_decision_line)
        .collect()
}

fn parse_decision_line(line: &str) -> Option<DecisionVerb> {
    let line = line.trim_start();
    let upper = line.to_ascii_uppercase();
    if !upper.starts_with("DECISION:") {
        return None;
    }
    let body = line["DECISION:".len()..].trim();
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

/// Judge a specialist's final text by its contract line. `Ok` on a pass;
/// the error carries the failure reason.
pub(crate) fn judge_output(role: Role, output: &str) -> Result<(), String> {
    // A delegation that never returned (budget spent, model failure) fails its
    // task regardless of role contract: the deliverable is unverified.
    if let Some(rest) = output.strip_prefix("DELEGATION FAILED: ") {
        return Err(rest.to_owned());
    }
    match role {
        Role::Validator => match judges::parse_verdict(output, "VALIDATION") {
            Some((true, _)) => Ok(()),
            Some((false, reason)) => Err(format!("validation failed: {reason}")),
            None => Err("validator produced no VALIDATION line".into()),
        },
        Role::Critic => match judges::parse_verdict(output, "CRITIQUE") {
            Some((true, _)) => Ok(()),
            Some((false, reason)) => Err(format!("critique failed: {reason}")),
            None => Err("critic produced no CRITIQUE line".into()),
        },
        Role::Repairer => match judges::parse_repair(output) {
            Some(("applied", _)) => Ok(()),
            Some((word, detail)) => Err(format!("repair {word}: {detail}")),
            None => Err("repairer produced no REPAIR line".into()),
        },
        Role::FailureAnalyst => {
            if judges::parse_diagnosis(output).is_some() {
                Ok(())
            } else {
                Err("failure analyst produced no DIAGNOSIS line".into())
            }
        },
        // Declarative roles pass when they produced nonempty output.
        _ => {
            if output.trim().is_empty() {
                Err("empty specialist output".into())
            } else {
                Ok(())
            }
        },
    }
}

/// Compact UTC timestamp for ledger ordering (`unix:<seconds>`). The ledger
/// only needs monotone-ish ordering; the harness logs carry full precision.
pub(crate) fn now_string() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("unix:{seconds}")
}

/// Prompt one agent, retrying transient provider failures up to `max_retries`
/// times. Transport- and server-side failures (HTTP 4xx/5xx wrapped in
/// `HttpError`, provider 500s) are retryable; a run that exhausted its budget
/// or was cancelled is not. `hook`, when set, rides every attempt (the guard
/// gateway) and a turn cap bounds the run.
pub(crate) async fn prompt_with_retries(
    agent: &rig::agent::Agent,
    prompt: &str,
    max_retries: u32,
    hook: Option<crate::guard_hook::GuardHook>,
    default_max_turns: usize,
) -> anyhow::Result<String> {
    use rig::completion::Prompt as _;
    let mut attempt = 0;
    loop {
        let mut request = agent.prompt(prompt.to_owned()).max_turns(default_max_turns);
        if let Some(hook) = hook.clone() {
            request = request.add_hook(hook);
        }
        match request.extended_details().await {
            Ok(details) => return Ok(details.output),
            Err(error) if attempt < max_retries && is_transient(&error) => {
                attempt += 1;
                tracing::warn!(
                    attempt,
                    max_retries,
                    error = %error,
                    "transient provider failure; retrying model call"
                );
            },
            Err(error) => return Err(error.into()),
        }
    }
}

/// Whether a prompt error looks transient: provider-side HTTP failures and
/// response-encoding hiccups. Budget exhaustion and cancellation are not.
fn is_transient(error: &rig::completion::PromptError) -> bool {
    match error {
        // Transport- and server-side only. `ResponseError` covers
        // deterministic failures (budget exhaustion, malformed output);
        // retrying one restarts the conversation from scratch and burns
        // the same budget again.
        rig::completion::PromptError::CompletionError(rig::completion::CompletionError::HttpError(_))
        | rig::completion::PromptError::CompletionError(rig::completion::CompletionError::ProviderError(_)) => true,
        _ => false,
    }
}

/// Read a file or empty string when absent.
fn read_or_empty(path: &Path) -> anyhow::Result<String> {
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

/// Load the state fresh for a phase transition write.
fn snapshot_phase(run_dir: &Path) -> anyhow::Result<blackboard::State> {
    blackboard::state::read(run_dir)?.ok_or_else(|| anyhow::anyhow!("state missing in {run_dir:?}"))
}

impl ManagerLoop {
    /// Harvest agent-written files from `workspace/meta/` into the run
    /// blackboard, then mirror the run state back. The run/ dir stays the
    /// source of truth; meta/ is the sandbox-visible read/write view.
    fn mirror_blackboard(&self) -> anyhow::Result<()> {
        let meta = self.workspace.root().join("meta");
        std::fs::create_dir_all(&meta)?;
        // Harvest: files specialists write through their sandbox.
        for (from, to) in [
            (meta.join("notes.md"), self.run_dir.join("notes.md")),
            (meta.join("plan.md"), self.run_dir.join("plan.md")),
        ] {
            if from.is_file() {
                std::fs::copy(&from, &to)?;
            }
        }
        // Mirror: run-state files agents must read.
        for (from, to) in [
            (self.run_dir.join("state.json"), meta.join("state.json")),
            (self.run_dir.join("tasks.json"), meta.join("tasks.json")),
            (self.run_dir.join("plan.md"), meta.join("plan.md")),
            (self.run_dir.join("ledgers/decisions.jsonl"), meta.join("decisions.jsonl")),
            (self.run_dir.join("ledgers/failures.jsonl"), meta.join("failures.jsonl")),
        ] {
            if from.is_file() {
                std::fs::copy(&from, &to)?;
            }
        }
        Ok(())
    }
}
