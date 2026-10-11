//! Path configuration.

use serde::Deserialize;

/// Input and output locations. Both are mandatory. The input directory is
/// read-only territory for the run, and all products land under the output
/// directory. Preflight rejects a blank value in either path (ADR 0032).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathsConfig {
    /// Directory holding the codebase to modernize. Never written to.
    pub input_dir: String,
    /// Directory receiving the modernized codebase and run artifacts.
    pub output_dir: String,
}
