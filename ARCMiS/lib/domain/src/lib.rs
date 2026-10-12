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
pub mod evidence;
pub mod gateway;
pub mod hierarchy;
pub mod mailbox;
pub mod mailboxes;
pub mod message;
pub mod module_phase;
pub mod module_state_machine;
pub mod module_transition;
pub mod phase;
pub mod phase_machine;
pub mod repository;
pub mod revision;
pub mod role;
pub mod team;
pub mod team_registry;
pub mod test_outcomes;
pub mod tool_call;
pub mod tool_verdict;
pub mod topic;
pub mod transition;

pub use blackboard::Blackboard;
pub use revision::Revision;
pub use evidence::Evidence;
pub use gateway::Gateway;
pub use tool_call::ToolCall;
pub use tool_verdict::ToolVerdict;
pub use hierarchy::{Role, Team, TeamRegistry};
pub use mailbox::Mailbox;
pub use mailboxes::Mailboxes;
pub use message::Message;
pub use module_phase::ModulePhase;
pub use module_state_machine::ModuleStateMachine;
pub use module_transition::ModuleTransition;
pub use phase::Phase;
pub use phase_machine::PhaseMachine;
pub use repository::RepositoryMetadata;
pub use test_outcomes::TestOutcomes;
pub use transition::Transition;
