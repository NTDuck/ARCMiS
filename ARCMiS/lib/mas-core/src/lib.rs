//! `mas_core` is the conductor runtime over the omp transport (ADR 0031).
//!
//! The conductor crate wires the composition crates together. It owns
//! the run lifetime, the transport handle, the task graph, the phase
//! machines, the artifact store, the router, the gateway, the
//! blackboard, the mailbox, the advisor, and the JEV service.
//!
//! Modules:
//! - `conductor` — one run: phases, dispatches, event loop.
//! - `context` — rolling context window with budget-driven compaction.
//! - `frame_sink` — maps decoded omp frames onto task-graph state changes.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod conductor;
pub mod context;
pub mod frame_sink;

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
pub use artifact::ContentHashRule;
pub use artifact::MigrationBriefSchema;
pub use artifact::Provenance;
pub use artifact::ProvenanceAction;
pub use artifact::ProvenanceChainRule;
pub use artifact::ProvenanceEntry;
pub use artifact::Schema;
pub use artifact::SchemaConformanceRule;
pub use artifact::SecurityAuditSchema;
pub use artifact::SourceMapSchema;
pub use artifact::TestReportSchema;
pub use artifact::TranslatedModuleSchema;
pub use artifact::ValidationError;
pub use artifact::ValidationRule;
pub use blackboard::Blackboard;
pub use blackboard::Revision;
pub use compact::compact_frames;
pub use compact::CompactionOutcome;
pub use compact::ContextFrame;
pub use conductor::Conductor;
pub use conductor::RunLimits;
pub use conductor::RunSummary;
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
pub use router::TRIAGE_QUESTION;
pub use taskgraph::FailureClass;
pub use taskgraph::GraphBudgets;
pub use taskgraph::InputRef;
pub use taskgraph::Kind;
pub use taskgraph::NodeId;
pub use taskgraph::Risk;
pub use taskgraph::StatusCounts;
pub use taskgraph::Strategy;
pub use taskgraph::TaskContract;
pub use taskgraph::TaskGraph;
pub use taskgraph::TaskId;
pub use taskgraph::TaskResult;
pub use taskgraph::TaskStatus;
pub use taskgraph::WorkItem;
pub use taskgraph::WriteScope;
pub use transport::OmpLaunchConfig;
pub use transport::OmpTransport;
pub use transport::Transport;
pub use transport::TransportError;
pub use verdict::Verdict;
