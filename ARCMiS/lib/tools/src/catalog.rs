//! Minimal tool catalog. Real behavior grows here.

use ::std::collections::BTreeMap;

/// Catalog of tools keyed by name.
#[derive(::core::fmt::Debug, ::core::default::Default)]
pub struct Catalog {
    tools: BTreeMap<::std::string::String, Tool>,
}

impl Catalog {
    /// Create an empty catalog.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
        }
    }

    /// Add a tool to the catalog.
    pub fn add(&mut self, name: impl Into<::std::string::String>) {
        let name = name.into();
        self.tools.insert(
            ::std::clone::Clone::clone(&name),
            Tool {
                name,
            },
        );
    }

    /// Look up a tool by name.
    #[must_use]
    pub fn get(&self, name: &str) -> ::core::option::Option<&Tool> {
        self.tools.get(name)
    }

    /// Number of tools in the catalog.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// True when the catalog holds no tools.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

#[derive(::core::fmt::Debug, ::core::clone::Clone, ::core::cmp::PartialEq, ::core::cmp::Eq)]
pub struct Tool {
    pub name: ::std::string::String,
}
