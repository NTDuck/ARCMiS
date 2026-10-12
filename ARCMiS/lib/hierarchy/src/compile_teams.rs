//! Map config teams onto typed teams.

use run_config::TeamConfig;

use crate::role::Role;
use crate::team::Team;

/// Map config teams onto typed teams, resolving every member name.
pub fn compile_teams(configs: &[TeamConfig]) -> Vec<Team> {
    configs.iter().map(resolve_team).collect()
}

/// Resolve one config team into a typed team.
fn resolve_team(config: &TeamConfig) -> Team {
    let members = config
        .members
        .iter()
        .filter_map(|name| {
            let role = Role::from_name(name);
            if role.is_none() {
                tracing::warn!(member = name, lead = config.lead, "team member is not a role. Drop it");
            }
            role
        })
        .collect::<Vec<Role>>();
    Team {
        lead: config.lead.clone(),
        members,
        turns: config.turns,
        stagnation_rounds: config.stagnation_rounds,
    }
}
