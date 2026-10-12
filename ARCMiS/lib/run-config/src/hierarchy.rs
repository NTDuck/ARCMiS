//! Static hierarchy layout (ADR 0026). One entry per tier-2 lead team.

use serde::Deserialize;

/// Tier-2 lead teams. A disabled hierarchy reproduces the single-tier
/// behavior exactly.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct HierarchyConfig {
    /// Whether the tier-2 lead teams are active.
    pub enabled: bool,
    /// One entry per lead. Each lead names its member specialists and its
    /// budgets. Members are a whole-fleet partition: a specialist that
    /// sits under two leads makes routing ambiguous. Preflight rejects
    /// it (ADR 0026).
    pub teams: Vec<TeamConfig>,
}

/// One tier-2 team: a lead plus the specialists it may dispatch.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamConfig {
    /// Lead role name, for example `migration-lead`.
    pub lead: String,
    /// Specialist role names this lead may dispatch.
    pub members: Vec<String>,
    /// Inner-loop round ceiling for the lead. A lead's own decisions
    /// consume this budget, not the whole run budget.
    pub turns: usize,
    /// Consecutive stagnant inner rounds before the lead escalates.
    pub stagnation_rounds: usize,
    /// Migration strategy for the team. Absent means `Parallel`
    /// (ADR 0038).
    #[serde(default)]
    pub strategy: Option<TeamStrategy>,
}

/// Migration strategy for one team (ADR 0038). The default is
/// `Parallel`; absent in the config file means `Parallel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TeamStrategy {
    /// Concurrent dispatch for independent modules.
    Parallel,
    /// Sequential dispatch respecting dependencies.
    Topological,
    /// Grouping for cycles.
    Grouped,
}

impl Default for TeamStrategy {
    fn default() -> Self {
        Self::Parallel
    }
}
