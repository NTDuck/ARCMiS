//! Agent orchestration domain for ARCMiS.

pub mod mas {
    //! Multi-agent system (MAS) methodology: 10 specialist roles plus an
    //! orchestrator over a persistent blackboard. The orchestrator crate
    //! drives the state machine. This crate owns the role definitions, the
    //! fleet model ladder, and the agent registry.
    pub mod fleet;
    pub mod leads;
    pub mod registry;
    pub mod roles;
}

pub mod util {
    //! Support modules for the agent domain. Nothing here implements an
    //! agent. The agents live in the crate root.
    pub mod config;
    pub mod provider;
}

pub use mas::fleet::Fleet;
pub use mas::leads::Team;
pub use mas::registry::MasAgents;
pub use mas::roles::Role;
pub use util::config::Config;
pub use util::config::TriagePolicy;
