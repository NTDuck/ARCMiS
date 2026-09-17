//! Agent orchestration domain for ARCMiS.

pub mod util {
    //! Support modules for the agent domain. Nothing here implements an
    //! agent; the agent lives in `crate::default`.
    pub mod config;
    pub mod measure;
    pub mod registry;
    pub mod sources;
}

pub mod default;

pub use default::{build, prompt, run};
pub use util::config::Config;
pub use util::measure::{measure, Measurement};
pub use util::registry::Registry;
