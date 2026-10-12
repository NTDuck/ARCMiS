//! `mas_domain` is the domain types and state machines for the MAS conductor.
//!
//! It bundles the static team registry, the run-level and per-module
//! phase machines, the tool-call gateway, the blackboard, the mailbox
//! switchboard, the repository metadata, and the test outcome
//! aggregation. The conductor and its collaborators depend on this
//! crate for every concern that names a domain concept.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod blackboard;
pub mod gateway;
pub mod hierarchy;
pub mod mailbox;
pub mod module_phase;
pub mod phase;
pub mod repository;
pub mod test_outcomes;

pub use blackboard::{Blackboard, Revision};
pub use gateway::{Gateway, ToolCall, ToolVerdict};
pub use hierarchy::{Role, Team, TeamRegistry};
pub use mailbox::{Mailbox, Mailboxes, Message};
pub use module_phase::{ModulePhase, ModuleStateMachine, ModuleTransition};
pub use phase::{Evidence, Phase, PhaseMachine, Transition};
pub use repository::RepositoryMetadata;
pub use test_outcomes::TestOutcomes;
