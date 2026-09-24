//! Registry: build the 11 role agents from the provider clients, the fleet
//! model assignment, and the tool instances. One agent per role; the
//! orchestrator wires the concrete tool set per delegation.

use rig::agent::Agent;
use rig::client::completion::CompletionClient;
use rig::client::AgentClientExt;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::mas::fleet::Fleet;
use crate::mas::roles::Role;
use crate::util::config::Run;

/// Built agents for one run.
#[derive(Clone)]
pub struct MasAgents {
    /// Agents per role.
    agents: Arc<BTreeMap<&'static str, Agent>>,
}

impl MasAgents {
    /// Look up one role's agent.
    #[must_use]
    pub fn agent(&self, role: Role) -> Option<&Agent> {
        self.agents.get(role.name())
    }
}

/// Build every role's agent from one client. Works for any provider whose
/// client implements `CompletionClient` (ollama native, netmind OpenAI
/// wire): the blanket `AgentClientExt` produces the same `AgentBuilder`.
pub fn build<C>(client: &C, fleet: &Fleet, run: &Run) -> anyhow::Result<MasAgents>
where
    C: CompletionClient,
    C::CompletionModel: 'static,
{
    let mut agents = BTreeMap::new();
    for role in Role::ALL {
        let model = fleet.model_for(role);
        let prompt = crate::mas::roles::prompt_text(role)?;
        let mut builder = client
            .agent(model)
            .preamble(&prompt)
            .temperature(run.temperature);
        if let Some(params) = extra_params(run) {
            builder = builder.additional_params(params);
        }
        agents.insert(role.name(), builder.build());
    }
    Ok(MasAgents {
        agents: Arc::new(agents),
    })
}

/// Provider-neutral extra params. Ollama models need the context window and
/// the think switch; the OpenAI wire has no counterpart, so the params ride
/// only when the run config says the provider is ollama.
fn extra_params(run: &Run) -> Option<serde_json::Value> {
    if run.provider_is_ollama() {
        Some(serde_json::json!({
            "num_ctx": run.num_ctx,
            "think": run.think,
        }))
    } else {
        None
    }
}
