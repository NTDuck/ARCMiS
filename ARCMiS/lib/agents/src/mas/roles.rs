//! Role catalog: the 10 specialist roles, their prompt files, and their
//! tool allowlists. The registry builds one agent per role from this table.

/// Specialist role. The router names one of these per delegation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// Manager: owns the plan and delegates.
    Manager,
    /// Reads the codebase; writes the source map.
    Analyst,
    /// Writes the migration brief.
    Architect,
    /// Produces dependency batches.
    Planner,
    /// Writes target-language modules for one batch.
    Translator,
    /// Judges one translated batch; read-only.
    Validator,
    /// Translates and adds tests; runs the test command.
    Tester,
    /// Classifies one failure.
    FailureAnalyst,
    /// End-of-run adversarial review; read-only.
    Critic,
    /// Fixes one diagnosed failure.
    Repairer,
    /// Recommends model promotion or demotion.
    FleetAnalyst,
}

impl Role {
    /// Every role in registry order (manager first).
    pub const ALL: [Role; 11] = [
        Role::Manager,
        Role::Analyst,
        Role::Architect,
        Role::Planner,
        Role::Translator,
        Role::Validator,
        Role::Tester,
        Role::FailureAnalyst,
        Role::Critic,
        Role::Repairer,
        Role::FleetAnalyst,
    ];

    /// Kebab-case name; matches the prompt file name and the router's answer.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Role::Manager => "manager",
            Role::Analyst => "analyst",
            Role::Architect => "architect",
            Role::Planner => "planner",
            Role::Translator => "translator",
            Role::Validator => "validator",
            Role::Tester => "tester",
            Role::FailureAnalyst => "failure-analyst",
            Role::Critic => "critic",
            Role::Repairer => "repairer",
            Role::FleetAnalyst => "fleet-analyst",
        }
    }

    /// Resolve a role from the router's one-word answer.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Role::ALL
            .into_iter()
            .find(|role| role.name() == name.trim().to_ascii_lowercase())
    }

    /// Tool names this role may call. The guard consults this list.
    #[must_use]
    pub fn allowed_tools(self) -> &'static [&'static str] {
        match self {
            // The manager reads state and writes plans/tasks/notes through
            // the blackboard files, not through code tools.
            Role::Manager => &["read", "search", "find", "ask"],
            // Analysts and judges read only.
            Role::Analyst | Role::Validator | Role::Critic | Role::FleetAnalyst => {
                &["read", "search", "find", "ast_grep"]
            },
            // The planner reads artifacts and writes plan.json through write.
            Role::Planner => &["read", "search", "find", "write"],
            // Movers touch the target tree.
            Role::Translator | Role::Repairer => {
                &["read", "write", "edit", "search", "find", "ast_grep", "ast_edit", "bash"]
            },
            // The tester also runs the test command through bash.
            Role::Tester => {
                &["read", "write", "edit", "search", "find", "ast_grep", "bash"]
            },
            // The failure analyst reads logs and code only.
            Role::FailureAnalyst => &["read", "search", "find", "bash"],
            Role::Architect => &["read", "search", "find", "write"],
        }
    }
}

/// Load the bundled prompt for one role. Public so the registry embeds the
/// same text the prompts directory carries.
pub fn prompt_text(role: Role) -> anyhow::Result<String> {
    let file = format!("{}/prompts/{}.md", env!("CARGO_MANIFEST_DIR"), role.name());
    let text = std::fs::read_to_string(&file)
        .map_err(|error| anyhow::anyhow!("read prompt {file}: {error}"))?;
    Ok(text)
}
