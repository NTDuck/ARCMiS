//! Agent orchestration domain for ARCMiS.

pub mod util {
    //! Support modules for the agent domain. Nothing here implements an
    //! agent. The agents live in the crate root.
    pub mod config;
    pub mod registry;
    pub mod sources;
}

pub mod monolith;
pub mod validator;

pub use monolith::{build as build_monolith, run as run_monolith};
pub use monolith::{MonolithRequest, MonolithResponse};
pub use util::config::Config;
pub use util::registry::Registry;
pub use validator::{build as build_validator, run as run_validator};
pub use validator::{ValidatorRequest, ValidatorResponse, ValidatorStepOutcome};
