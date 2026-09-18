//! Minimal agent registry. Real behavior grows here. The shape holds the
//! dependency direction: agents may use tools, never the reverse.

use std::collections::BTreeMap;
use std::sync::Arc;

/// Registry of agents keyed by name.
#[derive(Debug, Default)]
pub struct Registry {
    agents: BTreeMap<String, Arc<Agent>>,
}

impl Registry {
    /// Create an empty registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            agents: BTreeMap::new(),
        }
    }

    /// Register an agent under `name`.
    pub fn register(&mut self, name: impl Into<String>) -> Arc<Agent> {
        let name = name.into();
        let agent = Arc::new(Agent {
            name: name.clone(),
        });
        self.agents.insert(name, agent.clone());
        agent
    }

    /// Look up an agent by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Arc<Agent>> {
        self.agents.get(name)
    }

    /// Number of registered agents.
    #[must_use]
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// True when the registry holds no agent.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

/// A registered agent identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    pub name: String,
}
