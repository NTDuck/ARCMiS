//! Knobs for one advisor.
//!
//! Every value bubbles from the call site (config rule): the advisor
//! never reads the environment.

use crate::omp_launch_config::OmpLaunchConfig;

/// Knobs for one advisor. Every value bubbles from the call site
/// (config rule): the advisor never reads the environment.
#[derive(Debug, Clone)]
pub struct AdvisorConfig {
    /// Master switch. A disabled advisor returns `Defer` for every
    /// consultation and never spawns a transport.
    pub enabled: bool,
    /// Maximum number of pending consultations. The advisor returns
    /// `BacklogFull` once the backlog reaches this size.
    pub sync_backlog: usize,
    /// Launch config for the advisor's transport. The caller sets
    /// `read_only: true` so the advisor cannot write files or run
    /// commands.
    pub launch: OmpLaunchConfig,
}

impl Default for AdvisorConfig {
    fn default() -> Self {
        // WHY read_only: the advisor answers questions, never edits
        // files. Disabling every built-in tool at the CLI level keeps
        // the session read-only even if a prompt asks for a write.
        let launch = OmpLaunchConfig {
            read_only: true,
            ..OmpLaunchConfig::default()
        };
        Self {
            enabled: false,
            sync_backlog: 16,
            launch,
        }
    }
}
