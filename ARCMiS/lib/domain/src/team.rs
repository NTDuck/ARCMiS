//! One tier-2 team: a lead plus the members it may dispatch.

use crate::role::Role;

/// One tier-2 team: a lead plus the members it may dispatch.
#[derive(Debug, Clone)]
pub struct Team {
    /// Lead role name. Matches the run config spelling.
    pub lead: String,
    /// Specialists this lead may dispatch.
    pub members: Vec<Role>,
    /// Inner-loop round ceiling for this lead.
    pub turns: usize,
    /// Stagnant inner rounds before escalation.
    pub stagnation_rounds: usize,
}
