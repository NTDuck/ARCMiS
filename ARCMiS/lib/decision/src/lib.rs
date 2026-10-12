//! `mas_decision` is the decision-making layer for the MAS conductor.
//!
//! It bundles the JEV decision service for failure triage and the
//! layered router that dispatches each work item with a typed decision.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod decision_source;
pub mod escalation_priority;
pub mod failure_triage;
pub mod jev;
pub mod jev_config;
pub mod next_handler;
pub mod retry_strategy;
pub mod route_basis;
pub mod route_decision;
pub mod router;

pub use decision_source::DecisionSource;
pub use escalation_priority::EscalationPriority;
pub use failure_triage::FailureTriage;
pub use jev::JevDecisionService;
pub use jev_config::JevConfig;
pub use next_handler::NextHandler;
pub use retry_strategy::RetryStrategy;
pub use route_basis::RouteBasis;
pub use route_decision::RouteDecision;
pub use router::{Router, TRIAGE_QUESTION};
