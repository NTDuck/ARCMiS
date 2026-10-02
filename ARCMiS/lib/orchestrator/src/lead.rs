//! Lead inner loop: the tier-2 orchestration round (ADR 0026). A lead
//! receives a scoped brief (task text plus the blackboard artifacts its
//! team needs), runs its own bounded decision loop over its team's
//! specialist roles, and returns the batch outcome to the orchestrator as
//! one task result. Snapcompact and the trace hook (tier 2) attach at
//! build time. the decision parser allows only the team's member roles, and
//! the dispatch guard runs at dispatch time.

use agents::util::config::MasConfig;
use blackboard::Ledger;
use blackboard::Workspace;

use crate::loop_::now_string;
use crate::loop_::parse_decisions;
use crate::round_triage::policy::gate;
use crate::round_triage::policy::RoundAction;
use crate::taskgraph::run_single;
use crate::taskgraph::TaskResult;

/// Compose the scoped brief: the task text plus the blackboard artifacts
/// the team needs. Deliberately NOT the orchestrator's history (ADR 0026:
/// the tier handoff is context management). Caps reuse the blackboard
/// plan/notes caps so a lead never reads more than the orchestrator would.
#[must_use]
fn brief(task: &str, config: &MasConfig, workspace: &Workspace) -> String {
    let meta = workspace.root().join("meta");
    let capped = |name: &str, cap: usize| -> String {
        match std::fs::read_to_string(meta.join(name)) {
            Ok(text) => {
                let kept: String = text.chars().take(cap).collect();
                if kept.chars().count() < text.chars().count() {
                    format!("{kept}\n...[truncated]")
                } else {
                    kept
                }
            },
            Err(_) => String::new(),
        }
    };
    let contract = std::fs::read_to_string(meta.join("run.json")).unwrap_or_default();
    format!(
        "BATCH BRIEF (tier-1 delegation; your team executes it end to end)\n\nTASK:\n{task}\n\nRUN \
         CONTRACT:\n{contract}\n\nPLAN:\n{}\n\nNOTES:\n{}",
        capped("plan.md", config.plan_cap),
        capped("notes.md", config.notes_cap),
    )
}

/// Run one lead's batch: the tier-1 -> tier-2 handoff. Returns the batch
/// verdict (passed) and every member dispatch result (tier 3). The loop
/// ends on `done`, the turn ceiling, the stagnation breaker, or a lead
/// model failure.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_lead_batch(
    agents: &agents::MasAgents,
    config: &MasConfig,
    workspace: &Workspace,
    ledger: &Ledger,
    jev_triage: &crate::jev_triage::JevTriage,
    phase: blackboard::Phase,
    task_id: &str,
    lead_name: &str,
    delegation: &crate::taskgraph::Delegation,
) -> TaskResult {
    use crate::loop_::prompt_with_retries;

    let Some(lead) = agents.lead(lead_name) else {
        tracing::warn!(lead = lead_name, "no agent built for lead; hierarchy misconfigured");
        return TaskResult {
            id: task_id.to_owned(),
            role: agents::Role::Orchestrator,
            passed: false,
            output: format!("no lead agent named {lead_name}"),
            tier: 1,
        };
    };
    let Some(team) = crate::hierarchy::teams(config).into_iter().find(|team| team.lead == lead_name) else {
        return TaskResult {
            id: task_id.to_owned(),
            role: agents::Role::Orchestrator,
            passed: false,
            output: format!("lead {lead_name} has no resolved team"),
            tier: 1,
        };
    };
    let stagnation_cap = team.stagnation_rounds.unwrap_or(config.stagnation_rounds).max(1);
    let mut transcript = String::new();
    let mut results = Vec::new();
    let mut stalled = 0usize;

    ledger
        .append_decision(&blackboard::Decision {
            at: now_string(),
            phase: format!("{phase:?}"),
            action: "delegate-lead".into(),
            detail: serde_json::json!({"lead": lead_name, "task": delegation.task, "id": task_id, "tier": 2}),
            reasoning: "tier-2 handoff".into(),
        })
        .ok();

    for _round in 0..team.turns {
        // Round 1 carries the full brief. Later rounds carry a reminder plus
        // the dispatch history: the brief text does not change, and resending
        // it every round grows the lead's prompt monotonically (observed: a
        // lead prompt grew 6k -> 16k chars over 16 rounds while the decision
        // stayed a one-line dispatch).
        let prompt = if _round == 0 {
            format!(
                "{}\n\nTEAM: you may delegate to one of: {}.\n\nDecide the next action. Answer with DECISION lines \
                 (plain words, no angle brackets):\nDECISION: delegate ROLE | TASK TEXT\nDECISION: done\nROLE is one \
                 of the team members above. You may emit several delegate lines in one round when each task touches \
                 DIFFERENT deliverable paths. The harness runs them concurrently. Tasks that mutate the same files in \
                 sequence are not independent: keep them in separate rounds. TASK TEXT stays under 40 words: name the \
                 deliverable and the files to touch. The brief above already carries the detail, so do not restate it.",
                brief(&delegation.task, config, workspace),
                team.member_names().join(", "),
            )
        } else {
            format!(
                "ROUND {} of your batch. The task and the brief stay as before. TEAM: you may delegate to one of: \
                 {}.\n\nDISPATCH RESULTS SO FAR:\n{}\n\nDecide the next action. Answer with DECISION lines (plain \
                 words, no angle brackets):\nDECISION: delegate ROLE | TASK TEXT\nDECISION: done\nROLE is one of the \
                 team members above. You may emit several delegate lines in one round when each task touches \
                 DIFFERENT deliverable paths. The harness runs them concurrently. Tasks that mutate the same files in \
                 sequence are not independent: keep them in separate rounds. TASK TEXT stays under 40 words. \
                 Reference the brief and prior results instead of restating them.",
                _round + 1,
                team.member_names().join(", "),
                transcript,
            )
        };
        let answer = match prompt_with_retries(lead, &prompt, config.max_repairs.max(1) as u32, None, team.turns).await
        {
            Ok(turn) => turn.judgeable(),
            Err(error) => {
                tracing::warn!(lead = lead_name, error = %error, "lead model call failed. The batch fails");
                return finish(ledger, task_id, lead_name, false, format!("lead failed: {error}"), results);
            },
        };
        match parse_lead_round(&answer, &team) {
            LeadRound::Done => {
                // Acceptance gate: the tier-1 task text names deliverables in
                // its `Acceptance:` line; a batch that declares done without
                // them fails instead of passing a half-delivered brief
                // (observed c15b3: t1 passed without meta/plan.md, which
                // starved the Planning phase and tripped the breaker).
                let missing = missing_acceptance_files(&delegation.task, workspace);
                let passed = results.iter().any(|result| result.passed) && missing.is_empty();
                let mut summary = if passed {
                    "batch complete".to_owned()
                } else if results.iter().any(|result| result.passed) {
                    format!("batch incomplete; acceptance files missing: {}", missing.join(", "))
                } else {
                    "lead declared done with no passing dispatch".to_owned()
                };
                if !missing.is_empty() {
                    ledger
                        .append_failure(&blackboard::Failure {
                            at: now_string(),
                            phase: format!("{phase:?}"),
                            category: "gate".into(),
                            root_cause: format!(
                                "lead batch done without acceptance deliverables: {}",
                                missing.join(", ")
                            ),
                            suggested_action: "dispatch a member to produce the missing files".into(),
                        })
                        .ok();
                    summary.push_str("; dispatch a member to produce them, or re-emit done after repair");
                }
                return finish(ledger, task_id, lead_name, passed, summary, results);
            },
            LeadRound::Dispatches(dispatches) => {
                if dispatches.is_empty() {
                    // Every line refused or no DECISION at all: the same
                    // stagnation path as before ADR 0029, no new state.
                    stalled += 1;
                    if stalled >= stagnation_cap {
                        tracing::warn!(
                            lead = lead_name,
                            "lead emitted no usable decision; stagnation breaker ends the batch"
                        );
                        return finish(
                            ledger,
                            task_id,
                            lead_name,
                            false,
                            "lead stagnation breaker tripped".into(),
                            results,
                        );
                    }
                    continue;
                }
                // Over-fanout tail: truncate at the execution site (no
                // in-code queue, ADR 0029). The truncated dispatches get a
                // transcript note so the lead re-emits them next round.
                let fanout = config.fanout.max(1);
                let queued: Vec<_> = dispatches.iter().skip(fanout).collect();
                for (role, task) in &queued {
                    transcript.push_str(&format!(
                        "- [deferred] {}: emitted but not dispatched this round (fanout cap {}): {}\n",
                        role.name(),
                        fanout,
                        task.chars().take(120).collect::<String>()
                    ));
                }
                let runnable: Vec<_> = dispatches.into_iter().take(fanout).collect();
                // Member dispatches run concurrently, bounded by fanout
                // (ADR 0029). Shared handles clone cheaply (Arc inside
                // MasAgents and Ledger, a path inside Workspace).
                let mut jobs: Vec<tokio::task::JoinHandle<TaskResult>> = Vec::new();
                for (role, task) in runnable {
                    let agents = agents.clone();
                    let config = config.clone();
                    let workspace = workspace.clone();
                    let ledger = ledger.clone();
                    let inner = crate::taskgraph::Delegation {
                        role: Some(role.name().to_owned()),
                        task,
                        depends_on: Vec::new(),
                        team: false,
                        lead: None,
                    };
                    // Tier 3: the member executes under the tier-1 dispatch
                    // policy (same judge/tester tool-call caps). The trace
                    // hook marks its records tier 3 via the role agent build.
                    let task_id = task_id.to_owned();
                    jobs.push(tokio::spawn(async move {
                        run_single(&agents, &config, &workspace, &ledger, phase, &task_id, &inner, 3).await
                    }));
                }
                let mut round_results: Vec<TaskResult> = Vec::new();
                for job in jobs {
                    let result = match job.await {
                        Ok(result) => result,
                        Err(join_error) => TaskResult {
                            id: task_id.to_owned(),
                            role: agents::Role::Orchestrator,
                            passed: false,
                            output: format!("member dispatch panicked: {join_error}"),
                            tier: 3,
                        },
                    };
                    round_results.push(result);
                }
                // Typed-decision triage (ADR 0027): one laya forward pass
                // classifies each dispatch result instead of burning a lead
                // LLM turn on it. Instrumentation plus (ADR 0028) the
                // enforce policy: a confident stop-or-harmful verdict ends
                // the batch early with a triage failure row. A fallback or
                // a disabled judge leaves the loop byte identical.
                for result in &round_results {
                    let consultation = record_triage(jev_triage, ledger, phase, result, task_id);
                    if jev_triage.policy() == agents::TriagePolicy::Enforce
                        && gate(jev_triage.policy(), &consultation) == RoundAction::Stop
                    {
                        let _ = ledger.append_failure(&blackboard::Failure {
                            at: now_string(),
                            phase: format!("{phase:?}"),
                            category: "triage".into(),
                            root_cause: format!(
                                "dispatch triage stop: {} task={task_id}",
                                consultation_verdict(&consultation).map_or_else(
                                    || "no verdict".to_owned(),
                                    |v| format!(
                                        "outcome={} action={} confidence={:.2}",
                                        v.outcome, v.action, v.confidence
                                    )
                                ),
                            ),
                            suggested_action: "inspect the jev_triage observations".into(),
                        });
                        return finish(
                            ledger,
                            task_id,
                            lead_name,
                            false,
                            "dispatch triage policy stopped the batch".into(),
                            results,
                        );
                    }
                }
                for result in round_results {
                    transcript.push_str(&format!(
                        "- [{}] {}: {}\n",
                        if result.passed {
                            "pass"
                        } else {
                            "fail"
                        },
                        result.role.name(),
                        result.output.chars().take(600).collect::<String>()
                    ));
                    stalled = if result.passed {
                        0
                    } else {
                        stalled + 1
                    };
                    results.push(result);
                }
                if transcript.chars().count() > config.notes_cap {
                    transcript = transcript.chars().skip(transcript.chars().count() - config.notes_cap).collect();
                }
            },
        }
    }
    tracing::warn!(lead = lead_name, turns = team.turns, "lead turn budget exhausted; batch fails");
    finish(ledger, task_id, lead_name, false, "lead turn budget exhausted".into(), results)
}

/// Consult the typed-decision triage judge on one dispatch result and land
/// the verdict in the ledger. Disabled judge or `Fallback` writes nothing:
/// the ledger stays byte-identical to the uninstrumented run. `fail` plus
/// `needs_review` is the round-1 observation the A/B reads.
fn record_triage(
    triage: &crate::jev_triage::JevTriage,
    ledger: &Ledger,
    phase: blackboard::Phase,
    result: &TaskResult,
    task_id: &str,
) -> crate::jev_triage::TriageConsultation {
    if !triage.is_enabled() {
        return crate::jev_triage::TriageConsultation::Fallback;
    }
    let role = result.role.name();
    let verdict = match triage.consult_delegation(role, &format!("{phase:?}"), result.passed, &result.output) {
        crate::jev_triage::TriageConsultation::Decided {
            verdict,
        } => verdict,
        crate::jev_triage::TriageConsultation::Fallback => {
            // Name the reason: a silent fallback makes the instrumentation
            // invisible exactly when it misbehaves.
            if let Err(error) = triage.explain_fallback(role, &format!("{phase:?}"), result.passed, &result.output) {
                tracing::debug!(role, task = task_id, error = %error, "jev triage fell back");
            } else {
                tracing::debug!(role, task = task_id, "jev triage fell back below the confidence threshold");
            }
            return crate::jev_triage::TriageConsultation::Fallback;
        },
    };
    tracing::info!(
        role,
        outcome = %verdict.outcome,
        risk = verdict.risk,
        urgency = verdict.urgency,
        confidence = verdict.confidence,
        "jev triage classified a member dispatch"
    );
    ledger
        .append_observation(&blackboard::Observation {
            at: now_string(),
            kind: "jev_triage".into(),
            detail: serde_json::json!({
                "role": role,
                "task": task_id,
                "tier": 3,
                "passed": result.passed,
                "outcome": verdict.outcome,
                "needs_review": verdict.needs_review,
                "risk": verdict.risk,
                "urgency": verdict.urgency,
                "action": verdict.action,
                "confidence": verdict.confidence,
            }),
        })
        .ok();
    crate::jev_triage::TriageConsultation::Decided {
        verdict,
    }
}

fn consultation_verdict(
    consultation: &crate::jev_triage::TriageConsultation,
) -> Option<crate::jev_triage::TriageVerdict> {
    match consultation {
        crate::jev_triage::TriageConsultation::Decided {
            verdict,
        } => Some(verdict.clone()),
        crate::jev_triage::TriageConsultation::Fallback => None,
    }
}
fn missing_acceptance_files(task: &str, workspace: &Workspace) -> Vec<String> {
    let Some(position) = task.find("Acceptance:") else {
        return Vec::new();
    };
    let clause = &task[position + "Acceptance:".len()..];
    let mut missing = Vec::new();
    for token in
        clause.split(|character: char| !(character.is_alphanumeric() || matches!(character, '/' | '.' | '_' | '-')))
    {
        if token.contains('/')
            && token.contains('.')
            && !token.starts_with('.')
            && !workspace.root().join(token).exists()
            && !missing.contains(&token.to_owned())
        {
            missing.push(token.to_owned());
        }
    }
    missing
}
fn finish(
    ledger: &Ledger,
    task_id: &str,
    lead_name: &str,
    passed: bool,
    summary: String,
    results: Vec<TaskResult>,
) -> TaskResult {
    let output = if results.is_empty() {
        summary
    } else {
        let mut text = summary;
        for result in &results {
            text.push_str(&format!(
                "\n- [{}] {}",
                if result.passed {
                    "pass"
                } else {
                    "fail"
                },
                result.role.name()
            ));
        }
        text
    };
    ledger
        .append_observation(&blackboard::Observation {
            at: now_string(),
            kind: if passed {
                "lead_batch_pass"
            } else {
                "lead_batch_fail"
            }
            .into(),
            detail: serde_json::json!({"lead": lead_name, "task": task_id, "tier": 2}),
        })
        .ok();
    TaskResult {
        id: task_id.to_owned(),
        role: agents::Role::Orchestrator,
        passed,
        output,
        tier: 2,
    }
}

/// One parsed lead round: either the batch ends, or the lead emitted a
/// list of member dispatches (possibly empty when every line was refused
/// or the answer carried no DECISION at all).
#[derive(Debug)]
pub(crate) enum LeadRound {
    /// The batch is complete (or unrecoverable). return to tier 1.
    Done,
    /// Member dispatches in emission order. Empty when the lead's answer
    /// held no usable dispatch: the caller counts one stagnation step.
    Dispatches(Vec<(agents::Role, String)>),
}

/// Parse every DECISION line in the lead's answer (ADR 0029). Only the
/// lead's own member roles pass: a cross-team dispatch, a lead-to-lead
/// dispatch (leads are not specialist roles, so they never resolve here),
/// or an empty task is refused with a warning and skipped. A `done` verb
/// wins over any delegate lines in the same answer. The stagnation
/// counter handles repeated empty rounds.
pub(crate) fn parse_lead_round(answer: &str, team: &agents::mas::leads::Team) -> LeadRound {
    let mut dispatches: Vec<(agents::Role, String)> = Vec::new();
    for decision in parse_decisions(answer) {
        match decision {
            crate::loop_::DecisionVerb::Delegate {
                role,
                task,
            } => {
                let resolved = role.as_deref().and_then(agents::Role::from_name);
                match resolved {
                    Some(role) if team.permits(role) && !task.trim().is_empty() => {
                        dispatches.push((role, task));
                    },
                    other => {
                        tracing::warn!(
                            lead = %team.lead,
                            role = other.map(agents::Role::name).unwrap_or("<unresolved>"),
                            "lead delegation outside its team or empty; refused (depth cap, ADR 0026)"
                        );
                        continue;
                    },
                }
            },
            crate::loop_::DecisionVerb::Done => return LeadRound::Done,
            _ => continue,
        }
    }
    LeadRound::Dispatches(dispatches)
}
