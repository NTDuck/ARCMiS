//! Layered routing with laya typed decisions and keyword fallback.
//!
//! Static hierarchy fanout governs tier-2 to tier-3 dispatch. Prefer laya
//! typed triage (ADR 0027) when a classifier is available. Keyword
//! routing stays the deterministic fallback (ADR 0031).

use domain::Role;
use laya::Answer;
use laya::QuestionKind;

use crate::decision_source::DecisionSource;
use crate::route_basis::RouteBasis;
use crate::route_decision::RouteDecision;

/// The question id this router expects the triage agent to answer.
pub const TRIAGE_QUESTION: &str = "verdict";

/// Layered router. The keyword layer always answers.
#[derive(Debug, Default)]
pub struct Router {
    /// Optional triage agent for typed decisions.
    triage: Option<laya::Agent>,
}

impl Router {
    /// A keyword-only router.
    pub fn keyword_only() -> Self {
        Self {
            triage: None,
        }
    }

    /// A router with a triage agent on top.
    pub fn with_triage(agent: laya::Agent) -> Self {
        Self {
            triage: Some(agent),
        }
    }

    /// Route a failure text. Triage runs first when present. Keyword
    /// routing answers otherwise.
    pub fn route_failure(&self, failure: &str) -> RouteDecision {
        if let Some(agent) = &self.triage {
            let questions = serde_json::json!({ TRIAGE_QUESTION: failure });
            if let Ok(prediction) = agent.predict(&serde_json::json!({}), &questions) {
                if let Some(answer) = prediction.answer(TRIAGE_QUESTION) {
                    if answer.kind == QuestionKind::Choice {
                        return Self::from_answer(answer);
                    }
                }
            }
        }
        let (role, keyword) = Self::route_by_keywords(failure);
        RouteDecision {
            role,
            basis: RouteBasis::Keyword(keyword.map_or(DecisionSource::KeywordMiss, |word| DecisionSource::Keyword {
                keyword: word,
            })),
        }
    }

    /// Map one choice answer onto a specialist.
    fn from_answer(answer: &Answer) -> RouteDecision {
        let verdict = answer.choice.clone().unwrap_or_default();
        let role = Self::role_for_verdict(&verdict);
        RouteDecision {
            role,
            basis: RouteBasis::Typed(DecisionSource::Triage {
                verdict,
                confidence: answer.confidence,
            }),
        }
    }

    /// Deterministic keyword routing. Order matters: earlier patterns win.
    fn route_by_keywords(failure: &str) -> (Role, Option<&'static str>) {
        let lowered = failure.to_ascii_lowercase();
        let rules: [(&str, Role); 6] = [
            ("borrow checker", Role::UpliftWorker),
            ("type mismatch", Role::UpliftWorker),
            ("lifetime", Role::UpliftWorker),
            ("test failure", Role::DifferentialTester),
            ("assertion", Role::DifferentialTester),
            ("linker", Role::FailureAnalyst),
        ];
        rules
            .into_iter()
            .find(|(needle, _)| lowered.contains(needle))
            .map_or((Role::FailureAnalyst, None), |(needle, role)| (role, Some(needle)))
    }

    /// Map a triage verdict to a specialist.
    fn role_for_verdict(verdict: &str) -> Role {
        match verdict {
            "repair" => Role::Repairer,
            "retry" | "translator" => Role::UpliftWorker,
            "test" => Role::DifferentialTester,
            "plan" => Role::StrategySelector,
            _ => Role::FailureAnalyst,
        }
    }
}
