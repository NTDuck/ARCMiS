//! Tier-2 lead roles: static team definitions for hierarchical
//! orchestration (ADR 0026). A lead is not a `Role`: it never appears in
//! the tier-1 router table, it only wraps a team of specialist roles plus
//! a prompt file. The layout comes from the config (`mas.hierarchy.teams`).
//! The constants here are prompt-text helpers, not a hardcoded team split.

use std::path::Path;

use crate::mas::roles::Role;

/// One configured team: lead name, member roles, inner-loop budgets.
#[derive(Debug, Clone)]
pub struct Team {
    /// Lead role name (for example `migration-lead`). Matches the prompt
    /// file name.
    pub lead: String,
    /// Specialist roles this lead may dispatch.
    pub members: Vec<Role>,
    /// Inner-loop round ceiling for the lead's own model calls.
    pub turns: usize,
    /// Consecutive unproductive inner rounds before the lead's loop
    /// breaks. None keeps the run-level `stagnation_rounds`.
    pub stagnation_rounds: Option<usize>,
}

impl Team {
    /// Whether the lead may delegate to this role.
    #[must_use]
    pub fn permits(&self, role: Role) -> bool {
        self.members.contains(&role)
    }

    /// Member role names for the lead's prompt.
    #[must_use]
    pub fn member_names(&self) -> Vec<&'static str> {
        self.members.iter().map(|role| role.name()).collect()
    }
}

/// Load the bundled prompt for one lead. Same convention as
/// `roles::prompt_text`: the file lives in the agents prompts directory.
pub fn prompt_text(lead: &str) -> anyhow::Result<String> {
    let file = format!("{}/prompts/{}.md", env!("CARGO_MANIFEST_DIR"), lead);
    let text = std::fs::read_to_string(&file).map_err(|error| anyhow::anyhow!("read prompt {file}: {error}"))?;
    Ok(text)
}

/// Verify every configured team before the run starts: the prompt file
/// must exist and every member must resolve to a specialist role. A
/// config typo fails at preflight, not mid-run.
pub fn validate(teams: &[Team]) -> anyhow::Result<()> {
    for team in teams {
        let prompt = format!("{}/prompts/{}.md", env!("CARGO_MANIFEST_DIR"), team.lead);
        let path = Path::new(&prompt);
        anyhow::ensure!(path.is_file(), "lead prompt missing: {}", path.display());
        anyhow::ensure!(!team.members.is_empty(), "team {} has no members", team.lead);
    }
    Ok(())
}
