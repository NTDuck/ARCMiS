//! Round loop: one orchestration round. The orchestrator agent (tier 1)
//! reads the blackboard, decides, and the harness executes the decision
//! mechanically (delegate, replan, escalate, done). Renamed from
//! `ManagerLoop` (ADR 0026).

use std::path::Path;
use std::path::PathBuf;

use agents::util::config::MasConfig;
use agents::MasAgents;
use agents::Role;
use blackboard::Decision;
use blackboard::Ledger;
use blackboard::Phase;
use blackboard::TaskList;
use blackboard::Workspace;

use crate::judges;
use crate::state_machine;

/// Outcome of one orchestrator round.
#[derive(Debug, Clone)]
pub enum RoundOutcome {
    /// The orchestrator delegated. the harness ran the specialist.
    Delegated {
        /// Role the work went to.
        role: Role,
        /// Task id.
        task: String,
        /// Specialist's final text.
        output: String,
    },
    /// The orchestrator rewrote the plan. `true` when a plan body was
    /// persisted (a productive replan, not a bare verb).
    Replanned {
        /// Plan body persisted to `plan.md` this round.
        plan_persisted: bool,
    },
    /// The orchestrator escalated (repeated failure).
    Escalated(String),
    /// The orchestrator declared the phase done.
    PhaseDone(Phase),
    /// The harness refused a decision (evidence gate). The orchestrator sees
    /// the refusal in the next round's failure tail.
    Refused,
    /// The orchestrator asked to stop the run.
    Finished(String),
}

/// The round-loop context: everything one round touches.
pub struct OrchestratorLoop {
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
    /// Typed-decision triage judge over member dispatch results (ADR 0027).
    /// Built once at the call site; a disabled section keeps it off.
    pub jev_triage: crate::jev_triage::JevTriage,
    /// Round counter.
    pub round: usize,
    /// Retries per model call on transient provider errors.
    pub max_retries: u32,
    /// The previous round was a replan. The next prompt then demands a
    /// delegation: the plan is persisted, the next action is work
    /// (v3r3: three replans in a row tripped the stagnation breaker).
    pub last_round_replanned: bool,
}

impl OrchestratorLoop {
    /// Run one round: read state, ask the orchestrator, execute the decision.
    pub async fn round(&mut self) -> anyhow::Result<RoundOutcome> {
        self.round += 1;
        let previous_replanned = std::mem::take(&mut self.last_round_replanned);
        let state = blackboard::state::read(&self.run_dir)?
            .ok_or_else(|| anyhow::anyhow!("run state missing; preflight must write it"))?;
        // Mirror the blackboard into workspace/meta so every role reads the
        // run state through its sandbox. run/ itself is outside the tool root.
        self.mirror_blackboard()?;
        let prompt = self.round_prompt(&state, previous_replanned)?;
        let orchestrator =
            self.agents.agent(Role::Orchestrator).ok_or_else(|| anyhow::anyhow!("orchestrator agent missing"))?;

        // The orchestrator answers with one verb line: `DECISION: <verb>
        // [args]`.
        let answer = prompt_with_retries(orchestrator, &prompt, self.max_retries, None, self.config.orchestrator_turns)
            .await?
            .judgeable();
        let decisions = parse_decisions(&answer);
        if decisions.is_empty() {
            return Err(anyhow::anyhow!("orchestrator gave no DECISION line: {answer:?}"));
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
                &self.jev_triage,
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
            // Every delegation queued behind its dependencies. The
            // orchestrator's next round sees the pending tasks.
            // Report replan, not a delegation with a fabricated
            // role. Queuing work is productive: the plan body was
            // persisted by the earlier replan that created the tasks.
            return Ok(RoundOutcome::Replanned {
                plan_persisted: true,
            });
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
            } => Ok(RoundOutcome::Replanned {
                plan_persisted: false,
            }),
            DecisionVerb::Replan => {
                // Persist the plan body the orchestrator wrote in its answer
                // (c15b3: the model emitted a full proposed plan.md and the
                // harness dropped it; the prompt promises this persistence).
                let plan = plan_section(&answer);
                let plan_persisted = plan.is_some();
                if let Some(plan_body) = plan {
                    std::fs::write(self.run_dir.join("plan.md"), plan_body.as_str())?;
                    self.mirror_blackboard()?;
                }
                self.ledger.append_decision(&Decision {
                    at: now_string(),
                    phase: format!("{:?}", state.phase),
                    action: "replan".into(),
                    detail: serde_json::json!({"round": self.round, "plan_persisted": plan_persisted}),
                    reasoning: "orchestrator rewrote the plan".into(),
                })?;
                self.last_round_replanned = plan_persisted;
                Ok(RoundOutcome::Replanned {
                    plan_persisted,
                })
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
                // A phase with blocked tasks is not done (v3r6: the
                // orchestrator declared Pilot done while two tasks sat
                // blocked; the refusal tail must steer it to unblock or
                // re-plan around them first).
                let tasks = blackboard::tasks::TaskList::new(&self.run_dir).read().unwrap_or_default();
                let blocked = tasks.iter().filter(|task| task.status == blackboard::tasks::TaskStatus::Blocked).count();
                if blocked > 0 {
                    self.ledger.append_failure(&blackboard::Failure {
                        at: now_string(),
                        phase: format!("{:?}", state.phase),
                        category: "gate".into(),
                        root_cause: format!(
                            "{blocked} blocked task(s) still in the run; done is not available while work is blocked"
                        ),
                        suggested_action: "re-plan around the blocked tasks or fix and retry them (DECISION: replan \
                                           with a PLAN: section, or delegate the fix)"
                            .into(),
                    })?;
                    return Ok(RoundOutcome::Refused);
                }
                // A phase advances on evidence: at least one completed
                // delegation since the phase began. A bare claim advances
                // nothing (observed: the walk Pilot -> Migration ->
                // Integration in 18 s with no validation work).
                if state.phase_delegations <= state.phase_delegation_watermark {
                    self.ledger.append_failure(&blackboard::Failure {
                        at: now_string(),
                        phase: format!("{:?}", state.phase),
                        category: "gate".into(),
                        root_cause: "done claimed with no completed delegation in this phase. Advance needs at least \
                                     one judged pass"
                            .into(),
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

    /// Compose the orchestrator's round prompt from the blackboard.
    fn round_prompt(&self, state: &blackboard::State, previous_replanned: bool) -> anyhow::Result<String> {
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
        // Grammar and ROLE list depend on the orchestration tier (ADR 0026):
        // with hierarchy on the round prompt names the leads (plus the
        // tier-1 advisory role) and drops the `team` verb, so the
        // orchestrator cannot even see a specialist name to send.
        let (role_list, team_verb, hierarchy_note) = if self.config.hierarchy.enabled {
            let teams = crate::hierarchy::teams(&self.config);
            let mut names: Vec<&str> = teams.iter().map(|team| team.lead.as_str()).collect();
            names.push(Role::FleetAnalyst.name());
            (
                names.join(", "),
                String::new(),
                " With hierarchy on, batch work goes to a team lead; the lead dispatches its specialists. Delegating \
                 to fleet-analyst directly is allowed (advisory only)."
                    .to_owned(),
            )
        } else {
            (
                "analyst, architect, planner, translator, validator, tester, failure-analyst, critic, repairer, \
                 fleet-analyst"
                    .to_owned(),
                "DECISION: delegate ROLE | TASK TEXT | team\n".to_owned(),
                String::new(),
            )
        };
        Ok(format!(
            "ROUND {}\n\nSTATE: phase={:?} batch={:?} model={}\n\nPLAN:\n{}\n\nNOTES:\n{}\n\nRECENT \
             FAILURES:\n{}\n\nTASKS:\n{}\n\nDecide the next action. Answer with one or more DECISION lines (plain \
             words, no angle brackets, no function-call syntax):\nDECISION: delegate ROLE | TASK TEXT\nDECISION: \
             delegate ROLE | TASK TEXT | after:t2,t3\n{team_verb}DECISION: replan\nDECISION: escalate | \
             REASON\nDECISION: done\nDECISION: finish | REASON\nROLE is one of: {role_list}.\nFor replan: write the \
             new plan under a `PLAN:` heading before the DECISION line; the harness persists that section as plan.md. \
             Empty-plan replans change nothing.\nYou MAY emit several delegate lines in one round when the tasks are \
             independent; the harness runs up to {fanout} of them in parallel and queues the rest. `after:` lists \
             task ids this one waits on.             {hierarchy_note}{replan_note}",
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
            role_list = role_list,
            team_verb = team_verb,
            hierarchy_note = hierarchy_note,
            replan_note = if previous_replanned {
                "\n\nThe previous round rewrote the plan and it is persisted. This round MUST delegate at least                  one task (DECISION: delegate ...); another replan is not a valid next action unless a delegation                  also happens."
            } else {
                ""
            },
        ))
    }
}

/// The orchestrator's decision verbs.
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

/// Parse every `DECISION:` line. the orchestrator may emit several delegates in
/// one round (the task-graph executor orders and gates them).
pub(crate) fn parse_decisions(answer: &str) -> Vec<DecisionVerb> {
    answer
        .lines()
        .filter(|line| line.trim_start().to_ascii_uppercase().starts_with("DECISION:"))
        .filter_map(parse_decision_line)
        .collect()
}

/// Extract a `PLAN:` section from the orchestrator's answer. The model
/// writes the new plan body under this heading (usually right before the
/// DECISION lines); the replan verb persists it as `plan.md`. Returns
/// `None` when the answer carries no plan section.
pub(crate) fn plan_section(answer: &str) -> Option<String> {
    let mut body = None;
    for marker in ["PLAN:", "PROPOSED PLAN"] {
        if let Some(start) = answer.find(marker) {
            let start = start + marker.len();
            // Cut at the first DECISION line or the next all-caps heading so
            // reasoning text after the plan does not leak into plan.md.
            let rest = &answer[start..];
            let end = rest
                .lines()
                .enumerate()
                .find(|(_, line)| {
                    let trimmed = line.trim_start();
                    trimmed.to_ascii_uppercase().starts_with("DECISION:")
                        || trimmed.ends_with(':') && trimmed.len() <= 40 && trimmed.to_ascii_uppercase() == trimmed
                })
                .map_or(rest.len(), |(index, _)| rest.lines().take(index).collect::<Vec<_>>().join("\n").len());
            let candidate = rest[..end].trim();
            if !candidate.is_empty() {
                body = Some(candidate.to_owned());
            }
        }
    }
    body
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

/// Judge a specialist's final text by its contract line. `Ok` marks a pass.
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
        Role::FailureAnalyst =>
            if judges::parse_diagnosis(output).is_some() {
                Ok(())
            } else {
                Err("failure analyst produced no DIAGNOSIS line".into())
            },
        // Declarative roles pass when they produced nonempty output.
        _ =>
            if output.trim().is_empty() {
                Err("empty specialist output".into())
            } else {
                Ok(())
            },
    }
}

/// Compact UTC timestamp for ledger ordering (`unix:<seconds>`). The ledger
/// only needs monotone-ish ordering. the harness logs carry full precision.
pub(crate) fn now_string() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("unix:{seconds}")
}

/// One completed agent turn: the final text plus the tool calls it carried.
/// A turn with executed tool calls is delivered work, not a missing answer: a
/// tool-only turn judges on the calls it made, not on its empty text.
#[derive(Debug, Clone)]
pub(crate) struct TurnOutput {
    pub text: String,
    pub tool_calls: Vec<String>,
}

impl TurnOutput {
    /// The judgeable output: text when present, else a tool-call marker.
    pub fn judgeable(&self) -> String {
        if !self.text.trim().is_empty() || self.tool_calls.is_empty() {
            return self.text.clone();
        }
        format!("[{} tool calls: {}]", self.tool_calls.len(), self.tool_calls.join(", "))
    }
}

/// Prompt one agent, retrying transient provider failures up to `max_retries`
/// times. Transport- and server-side failures (HTTP 4xx/5xx wrapped in
/// `HttpError`, provider 500s) are retryable. a run that exhausted its budget
/// or was cancelled is not. `hook`, when set, rides every attempt (the guard
/// gateway) and a turn cap bounds the run.
pub(crate) async fn prompt_with_retries(
    agent: &rig::agent::Agent,
    prompt: &str,
    max_retries: u32,
    hook: Option<crate::guard_hook::GuardHook>,
    default_max_turns: usize,
) -> anyhow::Result<TurnOutput> {
    use rig::completion::Prompt as _;
    let mut attempt = 0;
    loop {
        let mut request = agent.prompt(prompt.to_owned()).max_turns(default_max_turns);
        if let Some(hook) = hook.clone() {
            request = request.add_hook(hook);
        }
        match request.extended_details().await {
            Ok(details) => {
                let tool_calls: Vec<String> = details
                    .content
                    .iter()
                    .filter_map(|part| match part {
                        rig::completion::AssistantContent::ToolCall(call) => Some(call.function.name.clone()),
                        _ => None,
                    })
                    .collect();
                return Ok(TurnOutput {
                    text: details.output,
                    tool_calls,
                });
            },
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
        // deterministic failures (budget exhaustion, malformed output).
        // Retrying one restarts the conversation from scratch and burns
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

impl OrchestratorLoop {
    /// Harvest agent-written files from `workspace/meta/` into the run
    /// blackboard, then mirror the run state back. The run/ dir stays the
    /// source of truth. meta/ is the sandbox-visible read/write view.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn judgeable_annotates_tool_only_turn() {
        let turn = TurnOutput {
            text: String::new(),
            tool_calls: vec!["write_file".into(), "bash".into()],
        };
        let output = turn.judgeable();
        assert!(output.contains("tool calls"), "marker missing: {output}");
        assert!(output.contains("write_file"));
        // The judged verdict: a tool-only turn is delivered work, not an
        // empty specialist output.
        assert_eq!(judge_output(Role::Tester, &output), Ok(()));
    }

    #[test]
    fn judgeable_keeps_text_output_unchanged() {
        let turn = TurnOutput {
            text: "wrote target/src/lib.rs".into(),
            tool_calls: vec!["write_file".into()],
        };
        assert_eq!(turn.judgeable(), "wrote target/src/lib.rs");
    }

    #[test]
    fn plan_section_cuts_at_decision_line() {
        let answer = "The batches changed.\n\nPLAN:\n# Plan\n- B1 core\n- B2 harness\n\nDECISION: replan";
        let plan = plan_section(answer).expect("plan section missing");
        assert!(plan.contains("# Plan"));
        assert!(plan.contains("- B2 harness"));
        assert!(!plan.contains("DECISION"), "plan leaked the decision line: {plan}");
    }

    #[test]
    fn plan_section_none_without_heading() {
        let answer = "The plan is stale.\n\nDECISION: replan";
        assert_eq!(plan_section(answer), None);
    }

    #[test]
    fn plan_section_prefers_plan_heading_over_proposed() {
        let answer = "Proposed plan below.\n\nPLAN:\n# Real plan\nB1\n\nDECISION: replan";
        let plan = plan_section(answer).expect("plan section missing");
        assert!(plan.contains("# Real plan"));
        assert!(!plan.contains("Proposed plan below"));
    }
}
