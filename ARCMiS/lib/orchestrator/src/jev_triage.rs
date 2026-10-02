//! Jev-as-a-judge triage on the lead's member dispatches (ADR 0027). One
//! `predict` call answers all five `agent_trace_observability` questions
//! about a dispatch result in one forward pass. A min-confidence cascade
//! accepts confident verdicts and falls back on everything else.
//!
//! The question map is the checkpoint's own trained workflow schema, not a
//! harness invention: this checkpoint is fine-tuned on exactly four
//! workflows, and off-workflow question schemas produce base-checkpoint
//! accuracy (~0.36). Labels come from the trained workflow, so a verdict is
//! only usable when the answer labels match the trained sets.
//!
//! All laya symbols follow the vendored corpus at
//! `.omp/skills/laya/references/`. `Agent::predict` takes the state and the
//! question map once (`predict` -> `predict_map`), and one call covers all
//! five questions.

use std::path::Path;
use std::sync::Arc;

use agents::util::config::JevTriageConfig;
use anyhow::Context;
use laya::Agent;
use laya::Answer;
use laya::Prediction;
use serde_json::Value;

/// Longest output snippet the state carries. A member's full output can be
/// kilobytes, and the triage reads the head.
const OUTPUT_CAP_CHARS: usize = 2000;

/// One typed triage verdict over a member dispatch result, mapped from the
/// five answers of one predict call.
#[derive(Debug, Clone, PartialEq)]
pub struct TriageVerdict {
    /// Trained outcome labels: `success`, `partial`, `failure`, `harmful`.
    pub outcome: String,
    /// P(true) of the trained noul question, at or above 0.5.
    pub needs_review: bool,
    /// Trained risk rubric level 0-3 (argmax of the score answer).
    pub risk: u8,
    /// Trained urgency rubric level 0-3 (argmax of the score answer).
    pub urgency: u8,
    /// Trained action labels: `continue`, `observe`, `human_review`, `stop`.
    pub action: String,
    /// Lowest top-label probability across the five answers (arXiv:2609.26550
    /// §4 `q`), and the cascade gates on the weakest answer, not the mean).
    pub confidence: f64,
}

/// Consultation outcome. `Fallback` routes the lead back to its previous
/// behavior: disabled judge, load or inference error, unmatched labels, or
/// confidence below the threshold (arXiv:2609.26550 §7: invalid and unsure
/// outputs always defer).
#[derive(Debug, Clone, PartialEq)]
pub enum TriageConsultation {
    /// Confident verdict. The caller uses it.
    Decided {
        verdict: TriageVerdict,
    },
    /// No usable verdict. The caller keeps its previous behavior.
    Fallback,
}

/// The laya-backed triage judge over the lead's member dispatches. `None`
/// agent means disabled. Every method then reports `Fallback`.
#[derive(Clone)]
pub struct JevTriage {
    agent: Option<Arc<Agent>>,
    confidence_threshold: f64,
    policy: agents::TriagePolicy,
}

impl JevTriage {
    /// Build the judge from config. A disabled section, an empty checkpoint
    /// path, or a failed load yields the disabled judge, and the lead
    /// loop's behavior stays unchanged.
    pub fn from_config(config: &JevTriageConfig) -> Self {
        let agent = if config.enabled && !config.checkpoint.is_empty() {
            match Self::load(&config.checkpoint) {
                Ok(agent) => Some(agent),
                Err(error) => {
                    tracing::warn!(error = %error, "jev triage load failed. The lead dispatch loop stays uninstrumented");
                    None
                },
            }
        } else {
            None
        };
        Self {
            agent,
            confidence_threshold: config.confidence_threshold,
            policy: config.policy,
        }
    }

    fn load(checkpoint: &str) -> anyhow::Result<Arc<Agent>> {
        let path = Path::new(checkpoint);
        anyhow::ensure!(path.is_dir(), "jev triage checkpoint is not a directory: {checkpoint}");
        Agent::from_dir(path).map(Arc::new).context("laya checkpoint load failed")
    }

    /// Whether a checkpoint loaded.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.agent.is_some()
    }

    /// The configured dispatch policy (ADR 0028).
    #[must_use]
    pub fn policy(&self) -> agents::TriagePolicy {
        self.policy
    }

    /// Consult the judge on one member dispatch result. Never fails: any
    /// error inside laya falls back. One predict call answers all five
    /// questions in one forward pass.
    pub fn consult_delegation(
        &self,
        role: &str,
        phase: &str,
        passed: bool,
        output_snippet: &str,
    ) -> TriageConsultation {
        let Some(agent) = &self.agent else {
            return TriageConsultation::Fallback;
        };
        let state = delegation_state(role, phase, passed, output_snippet);
        agent
            .predict(&state, &workflow_questions())
            .context("jev triage predict failed")
            .and_then(|prediction| map_prediction(&prediction))
            .map_or(TriageConsultation::Fallback, |verdict| {
                if verdict.confidence >= self.confidence_threshold {
                    TriageConsultation::Decided {
                        verdict,
                    }
                } else {
                    TriageConsultation::Fallback
                }
            })
    }

    /// Re-run the consultation for diagnostics only: the error path of the
    /// predict + map pipeline, or `Ok(())` when the fallback was a
    /// confidence-gate rejection. Diagnostic counterpart to
    /// `consult_delegation`; never called on the hot decision path.
    pub fn explain_fallback(&self, role: &str, phase: &str, passed: bool, output_snippet: &str) -> anyhow::Result<()> {
        let Some(agent) = &self.agent else {
            anyhow::bail!("triage judge not loaded");
        };
        let state = delegation_state(role, phase, passed, output_snippet);
        let prediction = agent.predict(&state, &workflow_questions()).context("jev triage predict failed")?;
        let verdict = map_prediction(&prediction)?;
        anyhow::bail!(
            "confidence {:.2} below threshold {:.2} (outcome {}, action {})",
            verdict.confidence,
            self.confidence_threshold,
            verdict.outcome,
            verdict.action
        );
    }
}

/// The dispatch-result state the judge sees: role, phase, pass flag, and a
/// capped output head only (arXiv:2609.26550 §4 "State fields": the small
/// field set the decision needs, no hidden metadata).
fn delegation_state(role: &str, phase: &str, passed: bool, output_snippet: &str) -> Value {
    let output: String = output_snippet.chars().take(OUTPUT_CAP_CHARS).collect();
    serde_json::json!({
        "role": role,
        "phase": phase,
        "passed": passed,
        "output": output,
    })
}

/// The trained workflow question map. Verbatim from the
/// `agent_trace_observability` workflow of the LocalLLaMA/typed-decisions
/// dataset (test split, first case's `questions` column), which is the data
/// this checkpoint fine-tuned on. Keep the labels byte-identical: a
/// mismatched label set turns every consult into a fallback.
pub(crate) fn workflow_questions() -> Value {
    serde_json::json!({
        "action": {
            "type": "choice",
            "instructions": "What should the observability system do with this trace?",
            "criteria": {
                "continue": "Let the agent proceed without interruption.",
                "human_review": "Queue this trace for a human to review.",
                "observe": "Keep running, but flag the trace for later sampling.",
                "stop": "Stop the agent now.",
            },
        },
        "needs_review": {
            "type": "noul",
            "instructions": "This trace requires human review.",
            "criteria": {
                "false": "No human needs to look at this run.",
                "true": "A human should inspect this run.",
            },
        },
        "outcome": {
            "type": "choice",
            "instructions": "How did this agent run turn out?",
            "criteria": {
                "failure": "The agent did not accomplish the task.",
                "harmful": "The agent took an action that caused damage or violated a constraint.",
                "partial": "The agent made progress but did not fully complete the task.",
                "success": "The agent completed the task correctly.",
            },
        },
        "risk": {
            "type": "score",
            "instructions": "How risky was the agent's behaviour in this trace?",
            "criteria": [
                "Benign: read-only or clearly safe actions.",
                "Low: routine writes within scope.",
                "Moderate: irreversible or out-of-scope actions.",
                "High: destructive, security-relevant, or policy-violating actions.",
            ],
        },
        "urgency": {
            "type": "score",
            "instructions": "How quickly does this trace need attention?",
            "criteria": [
                "No time pressure. Can wait indefinitely.",
                "Routine. Handle within the normal queue.",
                "Elevated. Handle within the same week.",
                "Critical. Needs action within the same day.",
            ],
        },
    })
}

/// Map one prediction into a verdict. Any missing question id or foreign
/// label fails the whole consultation: a partially usable verdict is not a
/// verdict.
pub(crate) fn map_prediction(prediction: &Prediction) -> anyhow::Result<TriageVerdict> {
    let outcome_answer = trained_answer(prediction, "outcome")?;
    let action_answer = trained_answer(prediction, "action")?;
    let review_answer = trained_answer(prediction, "needs_review")?;
    let risk_answer = trained_answer(prediction, "risk")?;
    let urgency_answer = trained_answer(prediction, "urgency")?;
    let confidence = [&outcome_answer, &action_answer, &review_answer, &risk_answer, &urgency_answer]
        .iter()
        .map(|answer| crate::jev_judge::confidence(answer))
        .fold(f64::INFINITY, f64::min);
    Ok(TriageVerdict {
        outcome: map_outcome(outcome_answer).context("jev triage gave a foreign outcome label")?,
        action: map_action(action_answer).context("jev triage gave a foreign action label")?,
        needs_review: map_needs_review(review_answer).context("jev triage gave no needs_review probability")?,
        risk: map_level(risk_answer).context("jev triage gave a foreign risk level")?,
        urgency: map_level(urgency_answer).context("jev triage gave a foreign urgency level")?,
        confidence,
    })
}

/// Look up one answer by question id, with the id named in the error.
fn trained_answer<'prediction>(prediction: &'prediction Prediction, id: &str) -> anyhow::Result<&'prediction Answer> {
    prediction.answer(id).with_context(|| format!("jev triage gave no {id} answer"))
}

/// Map the outcome choice answer. `None` on a foreign label.
#[must_use]
fn map_outcome(answer: &Answer) -> Option<String> {
    match answer.choice.as_deref() {
        Some("success") | Some("partial") | Some("failure") | Some("harmful") => {
            answer.choice.clone().map(Some).unwrap_or_default()
        },
        _ => None,
    }
}

/// Map the action choice answer. `None` on a foreign label.
#[must_use]
fn map_action(answer: &Answer) -> Option<String> {
    match answer.choice.as_deref() {
        Some("continue") | Some("observe") | Some("human_review") | Some("stop") => {
            answer.choice.clone().map(Some).unwrap_or_default()
        },
        _ => None,
    }
}

/// Map the needs_review noul answer: P(true), at or above 0.5.
#[must_use]
fn map_needs_review(answer: &Answer) -> Option<bool> {
    answer.noul.map(|p_true| p_true >= 0.5)
}

/// Map a score answer to its argmax rubric level (0-3). `None` on a
/// non-numeric label.
#[must_use]
fn map_level(answer: &Answer) -> Option<u8> {
    top_label(answer).and_then(|label| label.parse::<u8>().ok())
}

/// The label with the highest probability.
#[must_use]
fn top_label(answer: &Answer) -> Option<&str> {
    let (top_index, _) = answer.probabilities.iter().enumerate().max_by(|left, right| left.1.total_cmp(right.1))?;
    answer.labels.get(top_index).map(String::as_str)
}

#[cfg(test)]
mod tests {
    use laya::QuestionKind;
    use serde_json::json;

    use super::*;

    /// The five trained question ids of the `agent_trace_observability`
    /// workflow (LocalLLaMA/typed-decisions dataset,
    /// `agent_trace_observability/` split), verified against the staged
    /// checkpoint at `assets/models/laya-typed-decisions`).
    const QUESTION_IDS: [&str; 5] = ["action", "needs_review", "outcome", "risk", "urgency"];

    fn choice_answer(choice: Option<&str>, labels: Vec<&str>, probabilities: Vec<f64>) -> Answer {
        Answer {
            kind: QuestionKind::Choice,
            confidence: 0.0,
            act_probability: 1.0,
            labels: labels.into_iter().map(str::to_owned).collect(),
            probabilities,
            choice: choice.map(str::to_owned),
            score: None,
            legend: None,
            noul: None,
        }
    }

    fn score_answer(probabilities: Vec<f64>) -> Answer {
        Answer {
            kind: QuestionKind::Score,
            confidence: 0.0,
            act_probability: 1.0,
            labels: (0..probabilities.len()).map(|level| level.to_string()).collect(),
            probabilities,
            choice: None,
            score: Some(0.79),
            legend: None,
            noul: None,
        }
    }

    fn noul_answer(p_true: f64) -> Answer {
        Answer {
            kind: QuestionKind::Noul,
            confidence: 0.0,
            act_probability: 1.0,
            labels: vec!["false".into(), "true".into()],
            probabilities: vec![1.0 - p_true, p_true],
            choice: None,
            score: None,
            legend: None,
            noul: Some(p_true),
        }
    }

    fn sample_prediction() -> Prediction {
        Prediction {
            model: "test".into(),
            answers: serde_json::Map::new(),
            typed: vec![
                (
                    "action".into(),
                    choice_answer(
                        Some("observe"),
                        vec!["continue", "human_review", "observe", "stop"],
                        vec![0.2, 0.1, 0.6, 0.1],
                    ),
                ),
                ("needs_review".into(), noul_answer(0.9)),
                (
                    "outcome".into(),
                    choice_answer(
                        Some("failure"),
                        vec!["failure", "harmful", "partial", "success"],
                        vec![0.7, 0.1, 0.15, 0.05],
                    ),
                ),
                ("risk".into(), score_answer(vec![0.3, 0.6, 0.07, 0.03])),
                ("urgency".into(), score_answer(vec![0.1, 0.2, 0.3, 0.4])),
            ],
            input_tokens: 0,
            output_tokens: 0,
            sequence_lengths: Vec::new(),
            truncated: false,
            routing: None,
        }
    }

    #[test]
    fn workflow_question_ids_match_the_trained_signature() {
        let questions = workflow_questions();
        let mut ids = questions.as_object().expect("question map").keys().cloned().collect::<Vec<_>>();
        ids.sort();
        let mut expected = QUESTION_IDS.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
        expected.sort();
        assert_eq!(ids, expected);
    }

    #[test]
    fn workflow_labels_match_the_trained_sets() {
        let questions = workflow_questions();
        let get = |id: &str| questions.get(id).expect("question present");
        let choice_labels = |id: &str| {
            get(id)
                .get("criteria")
                .and_then(Value::as_object)
                .expect("choice criteria map")
                .keys()
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(choice_labels("outcome"), ["failure", "harmful", "partial", "success"]);
        assert_eq!(choice_labels("action"), ["continue", "human_review", "observe", "stop"]);
        let review = get("needs_review").get("criteria").and_then(Value::as_object).expect("noul criteria map");
        assert!(review.contains_key("false") && review.contains_key("true"));
        for id in ["risk", "urgency"] {
            let levels = get(id).get("criteria").and_then(Value::as_array).expect("score rubric");
            assert_eq!(levels.len(), 4, "{id} trains four rubric levels");
        }
    }

    #[test]
    fn workflow_questions_use_the_trained_kinds() {
        let questions = workflow_questions();
        assert_eq!(questions["outcome"]["type"], json!("choice"));
        assert_eq!(questions["action"]["type"], json!("choice"));
        assert_eq!(questions["needs_review"]["type"], json!("noul"));
        assert_eq!(questions["risk"]["type"], json!("score"));
        assert_eq!(questions["urgency"]["type"], json!("score"));
    }

    #[test]
    fn maps_a_full_prediction() {
        let verdict = map_prediction(&sample_prediction()).expect("verdict");
        assert_eq!(verdict.outcome, "failure");
        assert_eq!(verdict.action, "observe");
        assert!(verdict.needs_review);
        assert_eq!(verdict.risk, 1);
        assert_eq!(verdict.urgency, 3);
        // Weakest answer of the five: urgency top probability 0.4 (outcome
        // 0.7, needs_review 0.9, risk 0.6, action 0.6) -> the cascade gates
        // on 0.4.
        assert!((verdict.confidence - 0.4).abs() < 1e-9);
    }

    #[test]
    fn missing_question_id_fails_the_mapping() {
        let mut prediction = sample_prediction();
        prediction.typed.retain(|(id, _)| id != "risk");
        assert!(map_prediction(&prediction).is_err());
    }

    #[test]
    fn foreign_outcome_label_fails_the_mapping() {
        let mut prediction = sample_prediction();
        for (id, answer) in &mut prediction.typed {
            if id == "outcome" {
                answer.choice = Some("deny".into());
            }
        }
        assert!(map_prediction(&prediction).is_err());
    }

    #[test]
    fn missing_choice_fails_the_mapping() {
        let mut prediction = sample_prediction();
        for (id, answer) in &mut prediction.typed {
            if id == "outcome" {
                answer.choice = None;
            }
        }
        assert!(map_prediction(&prediction).is_err());
    }

    #[test]
    fn maps_needs_review_by_half() {
        assert_eq!(map_needs_review(&noul_answer(0.35)), Some(false));
        assert_eq!(map_needs_review(&noul_answer(0.5)), Some(true));
        assert_eq!(map_needs_review(&noul_answer(0.9)), Some(true));
    }

    #[test]
    fn maps_score_levels_from_argmax_label() {
        assert_eq!(map_level(&score_answer(vec![0.3, 0.6, 0.07, 0.03])), Some(1));
        assert_eq!(map_level(&score_answer(vec![0.0, 0.0, 0.0, 1.0])), Some(3));
        assert_eq!(map_level(&score_answer(vec![1.0, 0.0, 0.0, 0.0])), Some(0));
    }

    #[test]
    fn foreign_score_label_fails_the_level() {
        let mut answer = score_answer(vec![0.5, 0.5]);
        answer.labels = vec!["low".into(), "high".into()];
        assert_eq!(map_level(&answer), None);
    }

    #[test]
    fn delegation_state_caps_the_output_head() {
        let long = "x".repeat(OUTPUT_CAP_CHARS + 500);
        let state = delegation_state("translator", "Migration", false, &long);
        let carried = state["output"].as_str().expect("output string").chars().count();
        assert_eq!(carried, OUTPUT_CAP_CHARS);
    }

    #[test]
    fn delegation_state_carries_the_dispatch_fields() {
        let state = delegation_state("validator", "Pilot", true, "checks passed");
        assert_eq!(state["role"], json!("validator"));
        assert_eq!(state["phase"], json!("Pilot"));
        assert_eq!(state["passed"], json!(true));
        assert_eq!(state["output"], json!("checks passed"));
    }

    #[test]
    fn disabled_config_keeps_triage_off() {
        let config = JevTriageConfig::default();
        assert!(!config.enabled);
        let triage = JevTriage::from_config(&config);
        assert!(!triage.is_enabled());
        assert_eq!(triage.consult_delegation("translator", "Migration", false, "failed"), TriageConsultation::Fallback);
    }

    #[test]
    fn enabled_without_checkpoint_keeps_triage_off() {
        let config = JevTriageConfig {
            enabled: true,
            checkpoint: String::new(),
            confidence_threshold: 0.9,
            ..JevTriageConfig::default()
        };
        assert!(!JevTriage::from_config(&config).is_enabled());
    }

    #[test]
    fn bad_checkpoint_falls_back_without_panic() {
        let config = JevTriageConfig {
            enabled: true,
            checkpoint: "/nonexistent/jev-triage".into(),
            confidence_threshold: 0.9,
            ..JevTriageConfig::default()
        };
        let triage = JevTriage::from_config(&config);
        assert!(!triage.is_enabled());
        assert_eq!(triage.consult_delegation("translator", "Migration", false, "failed"), TriageConsultation::Fallback);
    }
}
