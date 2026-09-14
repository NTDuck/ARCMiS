//! Minimal agent registry. Real behavior grows here. The shape holds the
//! dependency direction: agents may use tools, never the reverse.

use ::std::collections::BTreeMap;
use ::std::sync::Arc;

/// A registered agent identity.
#[derive(::core::fmt::Debug, ::core::clone::Clone, ::core::cmp::PartialEq, ::core::cmp::Eq)]
pub struct Agent {
    pub name: ::std::string::String,
}

/// Registry of agents keyed by name.
#[derive(::core::fmt::Debug, ::core::default::Default)]
pub struct Registry {
    agents: BTreeMap<::std::string::String, Arc<Agent>>,
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
    pub fn register(&mut self, name: impl Into<::std::string::String>) -> Arc<Agent> {
        let name = name.into();
        let agent = Arc::new(Agent {
            name: ::std::clone::Clone::clone(&name),
        });
        self.agents.insert(name, ::core::clone::Clone::clone(&agent));
        agent
    }

    /// Look up an agent by name.
    #[must_use]
    pub fn get(&self, name: &str) -> ::core::option::Option<&Arc<Agent>> {
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
