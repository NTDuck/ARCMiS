//! `mas_core` is the conductor runtime over the omp transport (ADR 0031).
//!
//! Modules:
//! - `error` — run failure classes.
//! - `hierarchy` — static team registry from the run config.
//! - `phase` — evidence-gated phase machine with regression edges.
//! - `taskgraph` — typed nodes, dependencies, budgets, stagnation.
//! - `router` — layered routing with laya typed decisions.
//! - `blackboard` — bounded topic revisions.
//! - `mailbox` — bounded direct-message queues.
//! - `module_phase` — per-module FSM (ADR 0039 section 2).
//! - `gateway` — deterministic tool-call gateway.
//! - `compact` — snapcompact-style context compaction.
//! - `frames` — wire frame vocabulary over the omp session.
//! - `transport` — Transport seam over the omp session.
//! - `advisor` — read-only OMP consultation handle (ADR 0036 section 6).
//! - `conductor` — one run: phases, dispatches, event loop.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod advisor;
pub mod artifact;
pub mod blackboard;
pub mod compact;
pub mod conductor;
pub mod context;
pub mod error;
pub mod frame_sink;
pub mod frames;
pub mod gateway;
pub mod hierarchy;
pub mod mailbox;
pub mod module_phase;
pub mod phase;
pub mod router;
pub mod taskgraph;
pub mod transport;

pub use advisor::AdvisorConfig;
pub use advisor::AdvisorConsultation;
pub use advisor::AdvisorError;
pub use advisor::AdvisorVerdict;
pub use advisor::OmpAdvisor;
pub use artifact::Artifact;
pub use artifact::ArtifactContent;
pub use artifact::ArtifactEntry;
pub use artifact::ArtifactId;
pub use artifact::ArtifactKind;
pub use artifact::ArtifactStore;
pub use artifact::Provenance;
pub use artifact::ProvenanceAction;
pub use artifact::ProvenanceEntry;
pub use artifact::Schema;
pub use artifact::ValidationError;
pub use artifact::ValidationRule;
pub use blackboard::Blackboard;
pub use blackboard::Revision;
pub use compact::compact_frames;
pub use compact::CompactionOutcome;
pub use compact::ContextFrame;
pub use conductor::Conductor;
pub use conductor::RunSummary;
pub use conductor::Verdict;
pub use error::ConductorError;
pub use gateway::Gateway;
pub use gateway::ToolCall;
pub use gateway::ToolVerdict;
pub use hierarchy::Role;
pub use hierarchy::Team;
pub use hierarchy::TeamRegistry;
pub use mailbox::Mailbox;
pub use mailbox::Mailboxes;
pub use mailbox::Message;
pub use module_phase::ModulePhase;
pub use module_phase::ModuleStateMachine;
pub use module_phase::ModuleTransition;
pub use phase::Evidence;
pub use phase::Phase;
pub use phase::PhaseMachine;
pub use phase::Transition;
pub use router::DecisionSource;
pub use router::RouteBasis;
pub use router::RouteDecision;
pub use router::Router;
pub use taskgraph::NodeId;
pub use taskgraph::TaskGraph;
pub use taskgraph::TaskStatus;
pub use taskgraph::WorkItem;
pub use transport::OmpLaunchConfig;
pub use transport::OmpTransport;
pub use transport::Transport;
pub use transport::TransportError;
