//! Run configuration schema for MAS modernization runs.
//!
//! One run is described by one mandatory YAML file. The CLI rejects every
//! invocation without an explicit config path: codebase-specific settings
//! must stay explicit per run, because a silent default would hide them
//! (ADR 0032). This crate models the schema and validates it at preflight.
//!
//! Modules: `schema` (top-level sections), `paths` (locations),
//! `hierarchy` (lead teams), `errors` (finding type),
//! `validation` (preflight checks), `load` (parse-then-validate bridge).

mod errors;
mod hierarchy;
mod load;
mod paths;
mod schema;
mod validation;

pub use errors::ValidationError;
pub use hierarchy::{HierarchyConfig, TeamConfig};
pub use load::parse_and_validate;
pub use paths::PathsConfig;
pub use schema::{
    BlackboardConfig, CompactionConfig, GuardConfig, LanguagePair, LanguagesConfig, LayaConfig, LimitsConfig,
    ModelConfig, RunName, SectionConfig,
};
pub use validation::expect_valid;
