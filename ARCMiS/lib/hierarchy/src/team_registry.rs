//! Static tier registry.

use std::collections::HashSet;

use run_config::TeamConfig;

use crate::compile_teams::compile_teams;
use crate::role::Role;
use crate::team::Team;

/// Static tier registry, immutable for a run's duration. A disabled
/// hierarchy yields one anonymous tier where every dispatch is direct
/// and reproduction of the single-tier behavior is exact (ADR 0031).
#[derive(Debug, Default)]
pub struct TeamRegistry {
    /// Teams in config order. One direct-dispatch team when disabled.
    teams: Vec<Team>,
}

impl TeamRegistry {
    /// Compile the registry from one config team set. Every member name
    /// must resolve to a role.
    pub fn compile(enabled: bool, configs: &[TeamConfig]) -> Self {
        if !enabled {
            // WHY an empty-team registry: a disabled hierarchy must
            // reproduce single-tier behavior exactly, so no team limits
            // dispatch and every caller routes directly.
            return Self::direct();
        }
        Self {
            teams: compile_teams(configs),
        }
    }

    /// A disabled hierarchy: one anonymous tier, direct dispatch only.
    pub fn direct() -> Self {
        Self {
            teams: Vec::new(),
        }
    }

    /// Whether the registry enforces team routing at all.
    pub fn is_direct(&self) -> bool {
        self.teams.is_empty()
    }

    /// All teams, in config order.
    pub fn teams(&self) -> &[Team] {
        &self.teams
    }

    /// Resolve one team member to its lead. `None` means direct dispatch:
    /// the member sits in no team, so tier 1 keeps the work.
    pub fn lead_of(&self, member: Role) -> Option<&str> {
        self.teams.iter().find(|team| team.members.contains(&member)).map(|team| team.lead.as_str())
    }

    /// Whether `lead` may dispatch to `member`.
    pub fn permits(&self, lead: &str, member: Role) -> bool {
        self.teams.iter().any(|team| team.lead == lead && team.members.contains(&member))
    }

    /// Reject members sitting under two leads. Run-config preflight owns
    /// this check for config errors (ADR 0026). The registry enforces it
    /// again so a hand-built team set cannot smuggle ambiguity in.
    pub fn validate_unique_memberships(&self) -> Result<(), Vec<&str>> {
        let mut duplicates = self
            .teams
            .iter()
            .flat_map(|team| team.members.iter().map(move |member| (team.lead.as_str(), *member)))
            .filter(|&(lead, member)| {
                self.teams.iter().any(|other| other.lead != lead && other.members.contains(&member))
            })
            .map(|(_, member)| member.name())
            .collect::<HashSet<&str>>()
            .into_iter()
            .collect::<Vec<&str>>();
        duplicates.sort_unstable();
        if duplicates.is_empty() {
            Ok(())
        } else {
            Err(duplicates)
        }
    }
}
