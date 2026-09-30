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
                 of the team members above. One dispatch per round; the harness runs it to completion before your \
                 next round. TASK TEXT stays under 40 words: name the deliverable and the files to touch; the brief \
                 above already carries the detail, so do not restate it.",
                brief(&delegation.task, config, workspace),
                team.member_names().join(", "),
            )
        } else {
            format!(
                "ROUND {} of your batch (task unchanged; the brief you already have). TEAM: you may delegate to one \
                 of: {}.\n\nDISPATCH RESULTS SO FAR:\n{}\n\nDecide the next action. Answer with DECISION lines (plain \
                 words, no angle brackets):\nDECISION: delegate ROLE | TASK TEXT\nDECISION: done\nROLE is one of the \
                 team members above. One dispatch per round; the harness runs it to completion before your next \
                 round. TASK TEXT stays under 40 words; reference the brief and prior results instead of restating \
                 them.",
                _round + 1,
                team.member_names().join(", "),
                transcript,
            )
        };
        let answer = match prompt_with_retries(lead, &prompt, config.max_repairs.max(1) as u32, None, team.turns).await
        {
            Ok(turn) => turn.judgeable(),
            Err(error) => {
                tracing::warn!(lead = lead_name, error = %error, "lead model call failed; batch fails");
                return finish(ledger, task_id, lead_name, false, format!("lead failed: {error}"), results);
            },
        };
        match parse_lead_decision(&answer, &team) {
            Some(LeadDecision::Done) => {
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
            Some(LeadDecision::Delegate {
                role,
                task,
            }) => {
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
                let result = run_single(agents, config, workspace, ledger, phase, task_id, &inner, 3).await;
                // Typed-decision triage (ADR 0027): one laya forward pass
                // classifies the dispatch result instead of burning a lead
                // LLM turn on it. Instrumentation only this round: a
                // fallback or a disabled judge leaves the loop byte
                // identical, and the verdict lands in the ledger for the
                // campaign A/B. Dispatch-policy changes are the follow-up.
                record_triage(jev_triage, ledger, phase, &result, task_id);
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
                if transcript.chars().count() > config.notes_cap {
                    transcript = transcript.chars().skip(transcript.chars().count() - config.notes_cap).collect();
                }
                stalled = if result.passed {
                    0
                } else {
                    stalled + 1
                };
                results.push(result);
            },
            None => {
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
) {
    if !triage.is_enabled() {
        return;
    }
    let role = result.role.name();
    let verdict = match triage.consult_delegation(role, &format!("{phase:?}"), result.passed, &result.output) {
        crate::jev_triage::TriageConsultation::Decided {
            verdict,
        } => verdict,
        crate::jev_triage::TriageConsultation::Fallback => {
            tracing::debug!(role, task = task_id, "jev triage fell back; no verdict recorded");
            return;
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
}

/// Workspace paths named in the task's `Acceptance:` clause that do not
/// exist yet. Recognizes `path/file.ext` tokens; tokens without a `/` or
/// an extension are prose and ignored. Absent clause -> empty set.
#[must_use]
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

/// Ledger the batch verdict and fold it into one tier-1-visible result.
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

/// One inner-loop decision.
pub(crate) enum LeadDecision {
    /// Dispatch one member role.
    Delegate {
        role: agents::Role,
        task: String,
    },
    /// The batch is complete (or unrecoverable). return to tier 1.
    Done,
}

/// Parse the lead's answer into one decision. Only the lead's own member
/// roles pass: a cross-team dispatch, a lead-to-lead dispatch (leads are
/// not specialist roles, so they never resolve here), or an empty task
/// reads as unusable output. The stagnation counter handles repeats.
pub(crate) fn parse_lead_decision(answer: &str, team: &agents::mas::leads::Team) -> Option<LeadDecision> {
    for decision in parse_decisions(answer) {
        match decision {
            crate::loop_::DecisionVerb::Delegate {
                role,
                task,
            } => {
                let resolved = role.as_deref().and_then(agents::Role::from_name);
                match resolved {
                    Some(role) if team.permits(role) && !task.trim().is_empty() =>
                        return Some(LeadDecision::Delegate {
                            role,
                            task,
                        }),
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
            crate::loop_::DecisionVerb::Done => return Some(LeadDecision::Done),
            _ => continue,
        }
    }
    None
}
