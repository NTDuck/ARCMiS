//! Agent orchestration domain for ARCMiS.

pub mod mas;

pub mod util {
    //! Support modules for the agent domain. Nothing here implements an
    //! agent. The agents live in the crate root.
    pub mod config;
    pub mod provider;
}

pub use mas::Fleet;
pub use mas::MasAgents;
pub use mas::Role;
pub use util::config::Config;
