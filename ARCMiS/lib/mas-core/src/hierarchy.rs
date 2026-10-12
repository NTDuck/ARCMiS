//! `hierarchy` is the static team registry compiled once per run (ADR 0031).
//!
//! Tier 1 holds the orchestrator plus the advisory fleet-analyst. Tier 2
//! holds the leads. Tier 3 holds the frozen specialist roster. The
//! registry maps each specialist to its lead and answers fanout checks.
//!
//! The roster grew from ten to twenty-two variants per ADR 0038. The
//! ten old variants stay for backward compatibility; they are marked
//! `#[deprecated]` and resolve through the alias table.

use std::collections::HashSet;

use run_config::TeamConfig;

/// One tier-3 specialist role. The roster is frozen: every value names a
/// role the run config may assign to a team.
#[allow(deprecated)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Reads the codebase and writes the source map.
    #[deprecated(since = "0.2.0", note = "use LegacyAnalyst")]
    Analyst,
    /// Writes the migration brief.
    #[deprecated(since = "0.2.0", note = "use TopologyAnalyst")]
    Architect,
    /// Produces dependency batches.
    #[deprecated(since = "0.2.0", note = "use StrategySelector")]
    Planner,
    /// Writes target-language modules for one batch.
    #[deprecated(since = "0.2.0", note = "use UpliftWorker")]
    Translator,
    /// Judges one translated batch. Read-only.
    #[deprecated(since = "0.2.0", note = "use IndependentReviewer")]
    Validator,
    /// Adds tests and runs the test command.
    #[deprecated(since = "0.2.0", note = "use DifferentialTester")]
    Tester,
    /// Classifies one failure.
    FailureAnalyst,
    /// Applies one suggested fix.
    Repairer,
    /// End-of-run adversarial review. Read-only.
    #[deprecated(since = "0.2.0", note = "use ArchitectureCritic")]
    Critic,
    /// Advises on model promotion or demotion.
    FleetAnalyst,
    /// Reads the codebase and writes the source map. Discovery team.
    LegacyAnalyst,
    /// Writes the topology graph. Discovery team.
    TopologyAnalyst,
    /// Extracts business rules from the source code. Semantics team.
    BusinessRulesExtractor,
    /// Audits citations against the source map. Semantics team.
    CitationAuditor,
    /// Selects the migration strategy per module. Planning team.
    StrategySelector,
    /// Critiques the architecture. Planning team.
    ArchitectureCritic,
    /// Designs the test plan. Planning team.
    TestEngineer,
    /// Schedules modules for migration. Migration team.
    ModuleScheduler,
    /// Translates modules under the selected strategy. Migration team.
    UpliftWorker,
    /// Transforms modules under the selected strategy. Migration team.
    TransformWorker,
    /// Reimagines modules under the selected strategy. Migration team.
    ReimagineWorker,
    /// Reviews translations independently. Verification team.
    IndependentReviewer,
    /// Runs differential tests. Verification team.
    DifferentialTester,
    /// Builds proofs of correctness. Verification team.
    ProofBuilder,
    /// Audits the translated code for security issues. Security team.
    SecurityAuditor,
    /// Reviews patches before they land. Security team.
    PatchReviewer,
}

impl Role {
    /// Every specialist, in registry order.
    #[allow(deprecated)]
    pub const ALL: [Role; 26] = [
        Role::Analyst,
        Role::Architect,
        Role::Planner,
        Role::Translator,
        Role::Validator,
        Role::Tester,
        Role::FailureAnalyst,
        Role::Repairer,
        Role::Critic,
        Role::FleetAnalyst,
        Role::LegacyAnalyst,
        Role::TopologyAnalyst,
        Role::BusinessRulesExtractor,
        Role::CitationAuditor,
        Role::StrategySelector,
        Role::ArchitectureCritic,
        Role::TestEngineer,
        Role::ModuleScheduler,
        Role::UpliftWorker,
        Role::TransformWorker,
        Role::ReimagineWorker,
        Role::IndependentReviewer,
        Role::DifferentialTester,
        Role::ProofBuilder,
        Role::SecurityAuditor,
        Role::PatchReviewer,
    ];

    /// Kebab-case name. Matches the run config member spelling.
    #[allow(deprecated)]
    pub fn name(self) -> &'static str {
        match self {
            Role::Analyst => "analyst",
            Role::Architect => "architect",
            Role::Planner => "planner",
            Role::Translator => "translator",
            Role::Validator => "validator",
            Role::Tester => "tester",
            Role::FailureAnalyst => "failure-analyst",
            Role::Repairer => "repairer",
            Role::Critic => "critic",
            Role::FleetAnalyst => "fleet-analyst",
            Role::LegacyAnalyst => "legacy-analyst",
            Role::TopologyAnalyst => "topology-analyst",
            Role::BusinessRulesExtractor => "business-rules-extractor",
            Role::CitationAuditor => "citation-auditor",
            Role::StrategySelector => "strategy-selector",
            Role::ArchitectureCritic => "architecture-critic",
            Role::TestEngineer => "test-engineer",
            Role::ModuleScheduler => "module-scheduler",
            Role::UpliftWorker => "uplift-worker",
            Role::TransformWorker => "transform-worker",
            Role::ReimagineWorker => "reimagine-worker",
            Role::IndependentReviewer => "independent-reviewer",
            Role::DifferentialTester => "differential-tester",
            Role::ProofBuilder => "proof-builder",
            Role::SecurityAuditor => "security-auditor",
            Role::PatchReviewer => "patch-reviewer",
        }
    }

    /// Resolve a role from its config name. Accepts both new names and
    /// old names through the alias table (ADR 0038).
    #[allow(deprecated)]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::from_name_with_alias(name)
    }

    /// Resolve a role from its config name, applying the alias table.
    /// The alias table takes precedence so old names resolve to the new
    /// role, not the deprecated old variant (ADR 0038).
    pub fn from_name_with_alias(name: &str) -> Option<Self> {
        let normalized = name.trim().to_ascii_lowercase();
        Self::alias_for(&normalized).or_else(|| Self::ALL.into_iter().find(|role| role.name() == normalized.as_str()))
    }

    /// Map an old config name to its new role. Returns `None` for names
    /// that are not aliases (ADR 0038).
    fn alias_for(name: &str) -> Option<Self> {
        match name {
            "analyst" => Some(Role::LegacyAnalyst),
            "architect" => Some(Role::TopologyAnalyst),
            "planner" => Some(Role::StrategySelector),
            "translator" => Some(Role::UpliftWorker),
            "validator" => Some(Role::IndependentReviewer),
            "tester" => Some(Role::DifferentialTester),
            "failure-analyst" => Some(Role::FailureAnalyst),
            "repairer" => Some(Role::Repairer),
            "critic" => Some(Role::ArchitectureCritic),
            "fleet-analyst" => Some(Role::FleetAnalyst),
            _ => None,
        }
    }
}

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
            .collect::<Vec<(&str, Role)>>()
            .into_iter()
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

/// Map config teams onto typed teams, resolving every member name.
fn compile_teams(configs: &[TeamConfig]) -> Vec<Team> {
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
