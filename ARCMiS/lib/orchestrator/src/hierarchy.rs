//! Hierarchical team resolution (ADR 0026). Maps the config's
//! `mas.hierarchy` section to typed teams and answers the two routing
//! questions: which lead owns a specialist role, and which agents exist
//! for a lead name. No team layout lives here. the config owns the split.

use std::collections::BTreeMap;

use agents::mas::leads::Team;
use agents::util::config::MasConfig;
use agents::Role;

/// Resolve the configured teams. Member names that fail to resolve to a
/// specialist role are dropped with a warning. `crate::hierarchy::validate`
/// catches a fully broken team before the run starts.
#[must_use]
pub fn teams(config: &MasConfig) -> Vec<Team> {
    config
        .hierarchy
        .teams
        .iter()
        .map(|entry| Team {
            lead: entry.lead.clone(),
            members: entry.members().into_iter().flatten().collect(),
            turns: entry.turns,
            stagnation_rounds: entry.stagnation_rounds,
        })
        .collect()
}

/// The lead whose team contains `role`, if any.
#[must_use]
pub fn lead_for(teams: &[Team], role: Role) -> Option<&Team> {
    teams.iter().find(|team| team.permits(role))
}

/// Preflight the hierarchy config: every team resolves members, every
/// member is a specialist role, and no specialist sits in two teams (a
/// shared member would make routing ambiguous). Returns the teams on
/// success so the caller builds agents once.
pub fn validate(config: &MasConfig) -> anyhow::Result<Vec<Team>> {
    let teams = teams(config);
    anyhow::ensure!(!teams.is_empty(), "hierarchy enabled with no teams");
    let mut seen = BTreeMap::new();
    for team in &teams {
        anyhow::ensure!(!team.members.is_empty(), "team {} resolved to no valid members", team.lead);
        for member in &team.members {
            anyhow::ensure!(
                seen.insert(*member, team.lead.clone()).is_none(),
                "role {} sits in two teams ({}, {}); routing would be ambiguous",
                member.name(),
                seen[member],
                team.lead
            );
        }
        agents::mas::leads::validate(std::slice::from_ref(team))?;
    }
    Ok(teams)
}
