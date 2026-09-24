//! Route a request to the checkpoint best suited to it.
//!
//! The English checkpoint does not gently degrade off English: on 20-option
//! MASSIVE intent it scores 0.100 on Hindi and 0.103 on Korean against 0.050
//! for random guessing, and reports high confidence while doing so. Script
//! detection is therefore the primary routing signal, and the Latin-script
//! language guess is secondary.
//!
//! `typed-decisions` is never selected automatically unless you opt in with
//! [`Router::auto_task_detection`] or ask for it by name: it is fine-tuned on
//! four specific synthetic workflows and should not be a silent default.

use laya_core::lang;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::Agent;
use crate::{Error, Result};

/// The bundle repository carrying all three checkpoints.
pub const BUNDLE_REPO: &str = "convaiinnovations/laya";

/// Which checkpoint to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModelName {
    English,
    Multilingual,
    TypedDecisions,
}

impl ModelName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::Multilingual => "multilingual",
            Self::TypedDecisions => "typed-decisions",
        }
    }

    /// Repository and optional subfolder inside the bundle repository.
    pub fn bundle_location(self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::English => (BUNDLE_REPO, None),
            Self::Multilingual => (BUNDLE_REPO, Some("multilingual")),
            Self::TypedDecisions => (BUNDLE_REPO, Some("typed-decisions")),
        }
    }

    /// The standalone repository for this checkpoint.
    pub fn standalone_repo(self) -> &'static str {
        match self {
            Self::English => "convaiinnovations/laya",
            Self::Multilingual => "convaiinnovations/laya-multilingual",
            Self::TypedDecisions => "convaiinnovations/laya-typed-decisions",
        }
    }

    /// Resolve a name or alias, accepting everything upstream accepts.
    pub fn parse(name: &str) -> Result<Self> {
        let key = name.trim().to_lowercase();
        match key.as_str() {
            "english" | "en" | "eng" | "laya" | "default" => Ok(Self::English),
            "multilingual" | "multi" | "ml" | "laya-multilingual" => Ok(Self::Multilingual),
            "typed-decisions" | "typed" | "typed_decisions" | "laya-typed-decisions"
            | "decisions" => Ok(Self::TypedDecisions),
            _ => Err(Error::Config(format!(
                "unknown model {name:?}; choose one of [\"english\", \"multilingual\", \"typed-decisions\"]"
            ))),
        }
    }
}

/// Question-id signatures of the four typed-decisions workflows.
const TYPED_DECISION_WORKFLOWS: [(&str, &[&str]); 4] = [
    (
        "agent_trace_observability",
        &["action", "needs_review", "outcome", "risk", "urgency"],
    ),
    (
        "customer_service",
        &["action", "category", "churn_risk", "needs_human", "urgency"],
    ),
    (
        "invoice_processing",
        &[
            "discrepancy_severity",
            "disposition",
            "duplicate",
            "matches_order",
            "urgency",
        ],
    ),
    (
        "security_incidents",
        &[
            "credential_compromise",
            "disposition",
            "severity",
            "true_positive",
            "urgency",
        ],
    ),
];

/// Name of the typed-decisions workflow whose question ids these are.
///
/// Requires an exact id-set match, so an unrelated schema that happens to
/// contain `urgency` is never captured.
pub fn match_typed_decisions_workflow(questions: &Map<String, Value>) -> Option<&'static str> {
    let ids: std::collections::BTreeSet<&str> = questions.keys().map(String::as_str).collect();
    TYPED_DECISION_WORKFLOWS
        .iter()
        .find(|(_, signature)| {
            signature.len() == ids.len() && signature.iter().all(|id| ids.contains(id))
        })
        .map(|(name, _)| *name)
}

/// The routing outcome: which model, why, and what was detected.
#[derive(Clone, Debug, Serialize)]
pub struct RouteDecision {
    pub model: String,
    pub repo: String,
    pub reason: String,
    pub detection: Option<lang::Detection>,
    pub workflow: Option<String>,
}

impl RouteDecision {
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

/// Lazily loads checkpoints and sends each request to the right one.
pub struct Router {
    standalone_repos: bool,
    default: ModelName,
    auto_task_detection: bool,
    max_loaded: usize,
    agents: HashMap<ModelName, Arc<Agent>>,
    order: Vec<ModelName>,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    pub fn new() -> Self {
        Self {
            standalone_repos: false,
            default: ModelName::English,
            auto_task_detection: false,
            max_loaded: 1,
            agents: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Use each checkpoint's own repository rather than the bundle.
    pub fn standalone_repos(mut self, standalone: bool) -> Self {
        self.standalone_repos = standalone;
        self
    }

    /// Checkpoint used when detection is inconclusive.
    pub fn default_model(mut self, model: ModelName) -> Self {
        self.default = model;
        self
    }

    /// Allow the typed-decisions checkpoint to be selected by workflow match.
    pub fn auto_task_detection(mut self, enabled: bool) -> Self {
        self.auto_task_detection = enabled;
        self
    }

    /// How many checkpoints stay resident; least-recently-used is evicted.
    pub fn max_loaded(mut self, max_loaded: usize) -> Self {
        self.max_loaded = max_loaded.max(1);
        self
    }

    fn repo_string(&self, model: ModelName) -> String {
        if self.standalone_repos {
            model.standalone_repo().to_string()
        } else {
            match model.bundle_location() {
                (repo, Some(sub)) => format!("{repo}/{sub}"),
                (repo, None) => repo.to_string(),
            }
        }
    }

    /// Register an already-built agent rather than loading a second copy.
    pub fn attach(&mut self, model: ModelName, agent: Arc<Agent>) {
        self.agents.insert(model, agent);
        self.touch(model);
        self.max_loaded = self.max_loaded.max(self.agents.len());
    }

    fn touch(&mut self, model: ModelName) {
        self.order.retain(|entry| *entry != model);
        self.order.push(model);
    }

    /// Checkpoints currently resident, least-recently-used first.
    pub fn loaded(&self) -> &[ModelName] {
        &self.order
    }

    /// Raise the residency cap so a preload is not immediately evicted.
    pub(crate) fn set_max_loaded_at_least(&mut self, count: usize) {
        self.max_loaded = self.max_loaded.max(count).max(1);
    }

    /// Free one checkpoint, or all of them.
    pub fn unload(&mut self, model: Option<ModelName>) {
        match model {
            Some(model) => {
                self.agents.remove(&model);
                self.order.retain(|entry| *entry != model);
            }
            None => {
                self.agents.clear();
                self.order.clear();
            }
        }
    }

    /// Fetch an attached agent, if present.
    pub fn get(&mut self, model: ModelName) -> Option<Arc<Agent>> {
        let agent = self.agents.get(&model).cloned();
        if agent.is_some() {
            self.touch(model);
        }
        agent
    }

    /// Decide which checkpoint to use, without loading or running anything.
    ///
    /// Precedence: explicit `model` > explicit `task` > detected workflow
    /// (opt-in) > explicit `lang` > detected script/language > default.
    pub fn route(
        &self,
        state: &Value,
        questions: Option<&Map<String, Value>>,
        model: Option<&str>,
        task: Option<&str>,
        language: Option<&str>,
    ) -> Result<RouteDecision> {
        let empty = Map::new();
        let questions = questions.unwrap_or(&empty);

        if let Some(model) = model {
            let chosen = ModelName::parse(model)?;
            return Ok(RouteDecision {
                model: chosen.as_str().into(),
                repo: self.repo_string(chosen),
                reason: format!("explicit model={model:?}"),
                detection: None,
                workflow: None,
            });
        }

        if let Some(task) = task {
            let normalised = task.to_lowercase().replace('-', "_");
            let chosen = if normalised == "typed_decisions" {
                ModelName::TypedDecisions
            } else {
                ModelName::parse(task)?
            };
            return Ok(RouteDecision {
                model: chosen.as_str().into(),
                repo: self.repo_string(chosen),
                reason: format!("explicit task={task:?}"),
                detection: None,
                workflow: None,
            });
        }

        let workflow = match_typed_decisions_workflow(questions);
        if let (Some(workflow), true) = (workflow, self.auto_task_detection) {
            return Ok(RouteDecision {
                model: ModelName::TypedDecisions.as_str().into(),
                repo: self.repo_string(ModelName::TypedDecisions),
                reason: format!("question ids match the {workflow:?} typed-decisions workflow"),
                detection: None,
                workflow: Some(workflow.into()),
            });
        }

        if let Some(language) = language {
            let base = language.to_lowercase();
            let base = base.split('-').next().unwrap_or("");
            let chosen = if matches!(base, "en" | "eng" | "english") {
                ModelName::English
            } else {
                ModelName::Multilingual
            };
            return Ok(RouteDecision {
                model: chosen.as_str().into(),
                repo: self.repo_string(chosen),
                reason: format!("explicit lang={language:?}"),
                detection: None,
                workflow: workflow.map(Into::into),
            });
        }

        let detection = lang::analyse(state);
        let (chosen, reason) = if detection.script == "unknown" {
            (
                self.default,
                format!(
                    "no letters detected in state; using default ({})",
                    self.default.as_str()
                ),
            )
        } else if detection.script != "latin" {
            (
                ModelName::Multilingual,
                format!(
                    "non-Latin script ({}, {:.0}% of letters); the English checkpoint cannot read it",
                    detection.script,
                    100.0 * detection.non_latin_fraction
                ),
            )
        } else if !detection.is_english {
            (
                ModelName::Multilingual,
                format!(
                    "Latin script but language looks like {:?}, not English",
                    detection.language.clone().unwrap_or_default()
                ),
            )
        } else {
            (ModelName::English, "English Latin text".to_string())
        };

        Ok(RouteDecision {
            model: chosen.as_str().into(),
            repo: self.repo_string(chosen),
            reason,
            detection: Some(detection),
            workflow: workflow.map(Into::into),
        })
    }

    /// Route, then answer every question on the chosen checkpoint.
    ///
    /// The chosen checkpoint must already be attached; see [`Router::attach`],
    /// or the `hub` feature for automatic downloads.
    pub fn predict(
        &mut self,
        state: &Value,
        questions: &Map<String, Value>,
    ) -> Result<crate::agent::Prediction> {
        let decision = self.route(state, Some(questions), None, None, None)?;
        let chosen = ModelName::parse(&decision.model)?;
        let agent = self.get(chosen).ok_or_else(|| {
            Error::Config(format!(
                "checkpoint {:?} is not loaded; attach it first",
                decision.model
            ))
        })?;
        self.evict();
        let mut prediction = agent.predict_map(state, questions)?;
        prediction.routing = Some(decision.to_value());
        Ok(prediction)
    }

    fn evict(&mut self) {
        while self.order.len() > self.max_loaded {
            let victim = self.order.remove(0);
            self.agents.remove(&victim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn questions(ids: &[&str]) -> Map<String, Value> {
        ids.iter()
            .map(|id| (id.to_string(), json!({"type": "noul", "instructions": "x"})))
            .collect()
    }

    #[test]
    fn aliases_resolve() {
        assert_eq!(ModelName::parse("en").unwrap(), ModelName::English);
        assert_eq!(ModelName::parse("ML").unwrap(), ModelName::Multilingual);
        assert_eq!(
            ModelName::parse("typed_decisions").unwrap(),
            ModelName::TypedDecisions
        );
        assert!(ModelName::parse("nope").is_err());
    }

    #[test]
    fn explicit_model_wins() {
        let router = Router::new();
        let decision = router
            .route(&json!("hello"), None, Some("multilingual"), None, None)
            .unwrap();
        assert_eq!(decision.model, "multilingual");
        assert!(decision.reason.contains("explicit model"));
    }

    #[test]
    fn non_latin_routes_to_multilingual() {
        let router = Router::new();
        let decision = router
            .route(&json!("Мой счёт был дважды списан"), None, None, None, None)
            .unwrap();
        assert_eq!(decision.model, "multilingual");
        assert!(decision.reason.contains("non-Latin script"));
    }

    #[test]
    fn english_routes_to_english() {
        let router = Router::new();
        let decision = router
            .route(
                &json!("I was charged twice and would like a refund please"),
                None,
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(decision.model, "english");
    }

    #[test]
    fn workflow_detection_is_opt_in() {
        let ids = questions(&["action", "category", "churn_risk", "needs_human", "urgency"]);
        let off = Router::new();
        assert_eq!(
            off.route(&json!("hello"), Some(&ids), None, None, None)
                .unwrap()
                .model,
            "english"
        );

        let on = Router::new().auto_task_detection(true);
        let decision = on
            .route(&json!("hello"), Some(&ids), None, None, None)
            .unwrap();
        assert_eq!(decision.model, "typed-decisions");
        assert_eq!(decision.workflow.as_deref(), Some("customer_service"));
    }

    #[test]
    fn partial_workflow_match_is_ignored() {
        let ids = questions(&["urgency", "category"]);
        assert_eq!(match_typed_decisions_workflow(&ids), None);
    }

    #[test]
    fn explicit_language_routes_without_detection() {
        let router = Router::new();
        let decision = router
            .route(&json!("hello"), None, None, None, Some("de-DE"))
            .unwrap();
        assert_eq!(decision.model, "multilingual");
        assert!(decision.detection.is_none());
    }

    #[test]
    fn unknown_script_uses_the_default() {
        let router = Router::new().default_model(ModelName::Multilingual);
        let decision = router
            .route(&json!("12345"), None, None, None, None)
            .unwrap();
        assert_eq!(decision.model, "multilingual");
        assert!(decision.reason.contains("default"));
    }
}
