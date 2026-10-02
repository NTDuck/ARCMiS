//! MAS run orchestrator: the phase state machine, the round loop, the
//! router, the tool guard, the judge parsers, the circuit breaker, and the
//! progress tracker.

pub mod breaker;
pub mod guard;
pub mod guard_hook;
pub mod hierarchy;
pub mod jev_judge;
pub mod jev_triage;
pub mod judges;
pub mod lead;
#[cfg(test)]
mod lead_parse_tests;
// `loop` is a Rust keyword, so the module carries a trailing underscore. The
// type inside is `OrchestratorLoop`, so no caller types the module name.
pub mod loop_;
pub mod progress;
pub mod round_triage;
pub mod router;
pub mod state_machine;
pub mod taskgraph;

pub use breaker::Breaker;
pub use breaker::BreakerState;
pub use guard::Guard;
pub use guard_hook::GuardHook;
pub use jev_judge::confidence;
pub use jev_judge::map_ask_answer;
pub use jev_judge::AskVerdict;
pub use jev_judge::Consultation;
pub use jev_judge::JevJudge;
pub use jev_triage::JevTriage;
pub use jev_triage::TriageConsultation;
pub use jev_triage::TriageVerdict;
pub use judges::parse_diagnosis;
pub use judges::parse_repair;
pub use judges::parse_verdict;
pub use judges::Diagnosis;
pub use judges::FailureCategory;
pub use loop_::OrchestratorLoop;
pub use loop_::RoundOutcome;
pub use progress::Progress;
pub use round_triage::policy::RoundAction;
pub use round_triage::RoundEvidence;
pub use round_triage::RoundTriage;
pub use state_machine::Advance;
pub use state_machine::Transition;
