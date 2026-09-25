//! Multi-agent system (MAS) methodology: 10 specialist roles plus a manager
//! over a persistent blackboard. The orchestrator crate drives the state
//! machine; this crate owns the role definitions, the fleet model ladder,
//! and the agent registry.

pub mod fleet;
pub mod registry;
pub mod roles;
pub mod trace;

pub use fleet::Fleet;
pub use registry::MasAgents;
pub use roles::Role;
