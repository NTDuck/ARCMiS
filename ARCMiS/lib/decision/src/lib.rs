//! `mas_decision` is the decision-making layer for the MAS conductor.
//!
//! It bundles the JEV decision service for failure triage and the
//! layered router that dispatches each work item with a typed decision.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod jev;
pub mod router;

pub use jev::{EscalationPriority, FailureTriage, JevConfig, JevDecisionService, NextHandler, RetryStrategy};
pub use router::{DecisionSource, RouteBasis, RouteDecision, Router, TRIAGE_QUESTION};
