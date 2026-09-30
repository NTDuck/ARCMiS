//! Registry: build the 11 role agents from the provider clients, the fleet
//! model assignment, and the tool instances. One agent per role. The
//! orchestrator wires the concrete tool set per delegation.

use std::collections::BTreeMap;
use std::sync::Arc;

use middleware::role_output_tokens::RoleOutputTokensHook;
use middleware::trace::TraceHook;
use middleware::TrailingUserMessageHook;
use middleware::CONTINUATION_QUERY;
use rig::agent::Agent;
use rig::client::completion::CompletionClient;
use rig::client::AgentClientExt;

use crate::mas::fleet::Fleet;
use crate::mas::roles::Role;
use crate::util::config::Run;

/// Built agents for one run.
#[derive(Clone)]
pub struct MasAgents {
    /// Agents per role.
    agents: Arc<BTreeMap<&'static str, Agent>>,
    /// Tier-2 lead agents per lead name. Empty in single-tier mode
    /// (`mas.hierarchy.enabled` false).
    leads: Arc<BTreeMap<String, Agent>>,
}

impl MasAgents {
    /// Look up one role's agent.
    #[must_use]
    pub fn agent(&self, role: Role) -> Option<&Agent> {
        self.agents.get(role.name())
    }

    /// Attach the tier-2 lead agents built by `build_leads`. Harness code
    /// calls this once after `build`. the map stays private so a lead agent
    /// is only reachable through the hierarchy path.
    pub fn attach_leads(&mut self, leads: BTreeMap<String, Agent>) {
        self.leads = Arc::new(leads);
    }

    /// Look up one tier-2 lead's agent by name.
    #[must_use]
    pub fn lead(&self, name: &str) -> Option<&Agent> {
        self.leads.get(name)
    }

    /// The configured lead names, in config order.
    #[must_use]
    pub fn lead_names(&self) -> Vec<String> {
        self.leads.keys().cloned().collect()
    }
}

/// Build every role's agent from one client. Works for any provider whose
/// client implements `CompletionClient` (ollama native, netmind OpenAI
/// wire): the blanket `AgentClientExt` produces the same `AgentBuilder`.
///
/// `tools_for_role` supplies the portable tool adapters for one role. each
/// adapter is registered on that role's agent only, so the model sees
/// exactly its allowlist. `trace_sink`, when set, receives one JSONL line
/// per model call, model response, tool call, and tool result.
pub fn build<C, F>(
    client: &C,
    fleet: &Fleet,
    run: &Run,
    tools_for_role: F,
    trace_sink: Option<Arc<middleware::trace::TraceSink>>,
    turns_for_role: impl Fn(Role) -> usize,
    snapcompact: Option<&crate::util::config::SnapcompactConfig>,
    role_output_tokens: &BTreeMap<String, u64>,
    role_think: &BTreeMap<String, bool>,
) -> anyhow::Result<MasAgents>
where
    C: CompletionClient,
    C::CompletionModel: 'static,
    F: Fn(Role) -> Vec<tools::portable::Named>,
{
    let mut agents = BTreeMap::new();
    let leads = BTreeMap::new();
    // Parse the BDF font once. the snapcompact hook shares it across roles.
    let snapcompact_font = Arc::new(snapcompact::load_font());
    for role in Role::ALL {
        let model = fleet.model_for(role);
        let prompt = crate::mas::roles::prompt_text(role)?;
        let mut builder = client
            .agent(model)
            .preamble(&prompt)
            .temperature(run.temperature)
            .max_tokens(run.max_output_tokens)
            .default_max_turns(turns_for_role(role));
        if let Some(sink) = &trace_sink {
            // Tier 1: the registry builds single-tier agents. Hierarchy mode
            // (ADR 0026) builds lead agents separately at tier 2 via
            // `build_leads`, which traces with tier 2.
            builder = builder.add_hook(TraceHook::new(role.name(), 1, sink.clone()));
        }
        // A role with an output override gets its ceiling patched onto every
        // call. roles absent from the map keep the run default.
        if let Some(ceiling) = role_output_tokens.get(role.name()) {
            builder = builder.add_hook(RoleOutputTokensHook::new(*ceiling));
        }
        builder = builder.add_hook(TrailingUserMessageHook);
        // Snapcompact for every agent (ADR 0022): threshold 0 disables. The
        // trailing query keeps the continuation hook's user-query guarantee
        // alive when compaction replaces the history (ollama 500 otherwise).
        if let Some(config) = snapcompact.filter(|config| config.threshold_tokens > 0) {
            builder = builder.add_hook(snapcompact::SnapcompactHook {
                threshold_tokens: config.threshold_tokens,
                options: snapcompact::CompactOptions {
                    keep_recent_tokens: config.keep_recent_tokens,
                    ..snapcompact::CompactOptions::default()
                },
                font: snapcompact_font.clone(),
                trailing_user_query: Some(CONTINUATION_QUERY.to_owned()),
            });
        }
        if let Some(params) = extra_params(run, role_think.get(role.name()).copied()) {
            builder = builder.additional_params(params);
        }
        // Fold the adapters through the public portable_dynamic_tool. the
        // first call transitions the builder into the tools state. A role
        // with an empty allowlist builds tool-free (the orchestrator).
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
        leads: Arc::new(leads),
    })
}

/// Build the tier-2 lead agents for hierarchical mode (ADR 0026). One
/// tool-free agent per team lead: like the tier-1 orchestrator, a lead
/// decides in DECISION text and the harness executes the dispatch, so no
/// tool allowlist exists to leak across the tier boundary. The lead sits
/// on the weakest ladder rung (specialist tier). `lead_model` overrides
/// per team when the config names one.
pub fn build_leads<C>(
    client: &C,
    fleet: &Fleet,
    run: &Run,
    teams: &[crate::mas::leads::Team],
    trace_sink: Option<Arc<middleware::trace::TraceSink>>,
    snapcompact: Option<&crate::util::config::SnapcompactConfig>,
    role_think: &BTreeMap<String, bool>,
) -> anyhow::Result<BTreeMap<String, Agent>>
where
    C: CompletionClient,
    C::CompletionModel: 'static,
{
    let mut leads = BTreeMap::new();
    let snapcompact_font = Arc::new(snapcompact::load_font());
    for team in teams {
        let model = fleet.model_for(Role::Translator).to_owned();
        let prompt = crate::mas::leads::prompt_text(&team.lead)?;
        let mut builder = client
            .agent(model)
            .preamble(&prompt)
            .temperature(run.temperature)
            .max_tokens(run.max_output_tokens)
            .default_max_turns(team.turns);
        if let Some(sink) = &trace_sink {
            builder = builder.add_hook(TraceHook::new(team.lead.clone(), 2, sink.clone()));
        }
        builder = builder.add_hook(TrailingUserMessageHook);
        if let Some(config) = snapcompact.filter(|config| config.threshold_tokens > 0) {
            builder = builder.add_hook(snapcompact::SnapcompactHook {
                threshold_tokens: config.threshold_tokens,
                options: snapcompact::CompactOptions {
                    keep_recent_tokens: config.keep_recent_tokens,
                    ..snapcompact::CompactOptions::default()
                },
                font: snapcompact_font.clone(),
                trailing_user_query: Some(CONTINUATION_QUERY.to_owned()),
            });
        }
        if let Some(params) = extra_params(run, role_think.get(&team.lead).copied()) {
            builder = builder.additional_params(params);
        }
        leads.insert(team.lead.clone(), builder.build());
    }
    Ok(leads)
}

/// Provider-neutral extra params. Ollama models need the context window and
/// the native think switch. The OpenAI wire has no `num_ctx` counterpart,
/// but the thinking switch still matters: ninfer ignores ollama-style keys
/// and reads a top-level `enable_thinking` bool instead (verified against
/// the local daemon 2026-10-02; `reasoning_effort` works too, `think` and
/// `thinking` do not). A per-role think override (`mas.role_think`) wins
/// over the run default on either wire.
fn extra_params(run: &Run, think_override: Option<bool>) -> Option<serde_json::Value> {
    if run.provider_is_ollama() {
        Some(serde_json::json!({
            "num_ctx": run.num_ctx,
            "think": think_override.unwrap_or(run.think),
        }))
    } else {
        Some(serde_json::json!({
            "enable_thinking": think_override.unwrap_or(run.think),
        }))
    }
}
