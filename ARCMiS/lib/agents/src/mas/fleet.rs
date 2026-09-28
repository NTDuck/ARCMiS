//! Fleet model configuration: the model ladder and per-role assignment the
//! router and fleet analyst mutate at run time. Values come from the config
//! file; nothing here is hardcoded.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use crate::mas::roles::Role;

/// The model ladder and per-role model assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fleet {
    /// Manager model id.
    pub manager_model: String,
    /// Specialist model ladder, weakest first. Promotion moves one rung up.
    pub ladder: Vec<String>,
    /// Current model per specialist role.
    pub role_models: BTreeMap<String, String>,
}

impl Fleet {
    /// Model for one role.
    #[must_use]
    pub fn model_for(&self, role: Role) -> &str {
        match role {
            Role::Manager => &self.manager_model,
            other => self
                .role_models
                .get(other.name())
                .map(String::as_str)
                .unwrap_or_else(|| self.ladder.first().map(String::as_str).unwrap_or("")),
        }
    }

    /// Promote one role one rung up the ladder. Returns the new model, or
    /// `None` when the role already sits on the strongest rung.
    pub fn promote(&mut self, role: Role) -> Option<String> {
        let current = self.model_for(role).to_owned();
        let index = self.ladder.iter().position(|model| *model == current)?;
        let next = self.ladder.get(index + 1)?.clone();
        self.assign(role, next.clone());
        Some(next)
    }

    /// Demote one role one rung down the ladder. Returns the new model, or
    /// `None` when the role already sits on the weakest rung.
    pub fn demote(&mut self, role: Role) -> Option<String> {
        let current = self.model_for(role).to_owned();
        let index = self.ladder.iter().position(|model| *model == current)?;
        let previous = if index == 0 {
            return None;
        } else {
            self.ladder.get(index - 1)?.clone()
        };
        self.assign(role, previous.clone());
        Some(previous)
    }

    /// Assign one role to a model id directly.
    pub fn assign(&mut self, role: Role, model: String) {
        match role {
            Role::Manager => self.manager_model = model,
            other => {
                self.role_models.insert(other.name().to_owned(), model);
            },
        }
    }
}
