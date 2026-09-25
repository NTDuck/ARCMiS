//! MAS run orchestrator: the phase state machine, the manager loop, the
//! router, the tool guard, the judge parsers, the circuit breaker, and the
//! progress tracker.

pub mod breaker;
pub mod guard;
pub mod guard_hook;
pub mod judges;
pub mod manager;
pub mod progress;
pub mod router;
pub mod state_machine;
pub mod taskgraph;

pub use breaker::Breaker;
pub use breaker::BreakerState;
pub use guard::Guard;
pub use guard_hook::GuardHook;
pub use judges::parse_diagnosis;
pub use judges::parse_repair;
pub use judges::parse_verdict;
pub use judges::Diagnosis;
pub use judges::FailureCategory;
pub use manager::ManagerLoop;
pub use manager::RoundOutcome;
pub use progress::Progress;
pub use state_machine::Advance;
pub use state_machine::Transition;
