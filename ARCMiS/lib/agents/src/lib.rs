//! Agent orchestration domain for ARCMiS.

pub mod util {
    //! Support modules for the agent domain. Nothing here implements an
    //! agent. The agents live in the crate root.
    pub mod config;
    pub mod registry;
    pub mod sources;
    pub mod task;
}

pub mod ledger;
pub mod monolith;
pub mod recode;
pub mod validator;

pub use ledger::{Ledger, LedgerResponse};
pub use monolith::Monolith;
pub use monolith::{MonolithRequest, MonolithResponse};
pub use recode::{Recode, RecodeResponse};
pub use util::config::Config;
pub use util::registry::Registry;
pub use validator::Validator;
pub use validator::{ValidatorRequest, ValidatorResponse, ValidatorStepOutcome};
