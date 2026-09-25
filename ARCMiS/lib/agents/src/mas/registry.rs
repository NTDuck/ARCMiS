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
///
/// `tools_for_role` supplies the portable tool adapters for one role; each
/// adapter is registered on that role's agent only, so the model sees
/// exactly its allowlist. `trace_sink`, when set, receives one JSONL line
/// per model call, model response, tool call, and tool result.
pub fn build<C, F>(
    client: &C,
    fleet: &Fleet,
    run: &Run,
    tools_for_role: F,
    trace_sink: Option<Arc<crate::mas::trace::TraceSink>>,
) -> anyhow::Result<MasAgents>
where
    C: CompletionClient,
    C::CompletionModel: 'static,
    F: Fn(Role) -> Vec<tools::portable::Named>,
{
    let mut agents = BTreeMap::new();
    for role in Role::ALL {
        let model = fleet.model_for(role);
        let prompt = crate::mas::roles::prompt_text(role)?;
        let mut builder = client
            .agent(model)
            .preamble(&prompt)
            .temperature(run.temperature)
            .max_tokens(run.max_output_tokens)
            .default_max_turns(run.max_turns);
        if let Some(sink) = &trace_sink {
            builder = builder.add_hook(crate::mas::trace::TraceHook::new(role.name(), sink.clone()));
        }
        if let Some(params) = extra_params(run) {
            builder = builder.additional_params(params);
        }
        // Fold the adapters through the public portable_dynamic_tool; the
        // first call transitions the builder into the tools state. A role
        // with an empty allowlist builds tool-free (the manager).
        let mut adapters = tools_for_role(role).into_iter();
        let agent = match adapters.next() {
            Some(first) => {
                let mut with_tools = builder.portable_dynamic_tool(first.tool);
                for named in adapters {
                    with_tools = with_tools.portable_dynamic_tool(named.tool);
                }
                with_tools.build()
            },
            None => builder.build(),
        };
        agents.insert(role.name(), agent);
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
