//! Cucumber suite for the agent registry.

use ::agents::registry::Agent;
use ::agents::Registry;
use ::cucumber::{given, then, when, World};
use ::std::sync::Arc;

#[derive(::core::fmt::Debug, ::core::default::Default)]
#[derive(::cucumber::World)]
pub struct AgentWorld {
    registry: Registry,
    last: ::core::option::Option<Arc<Agent>>,
}

#[given("an empty registry")]
async fn empty_registry(w: &mut AgentWorld) {
    w.registry = Registry::new();
}

#[when(expr = "I register the agent {string}")]
async fn register_agent(w: &mut AgentWorld, name: String) {
    let agent = w.registry.register(name);
    w.last = ::core::option::Option::Some(agent);
}

#[then(expr = "the registry holds {int} agent")]
async fn holds_agents(w: &mut AgentWorld, expected: usize) {
    ::core::assert_eq!(w.registry.len(), expected);
}

#[then(expr = "the agent {string} is present")]
async fn agent_present(w: &mut AgentWorld, name: String) {
    ::core::assert!(w.registry.get(&name).is_some());
}

#[then(expr = "the agent {string} is absent")]
async fn agent_absent(w: &mut AgentWorld, name: String) {
    ::core::assert!(w.registry.get(&name).is_none());
}

#[::tokio::main]
async fn main() {
    AgentWorld::run("tests/features").await;
}
