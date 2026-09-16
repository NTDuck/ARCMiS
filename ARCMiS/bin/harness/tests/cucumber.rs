//! Cucumber suite for harness wiring.

use ::agents::Registry;
use ::cucumber::{given, then, World};
use ::tools::Catalog;

#[derive(::core::fmt::Debug, ::core::default::Default)]
#[derive(::cucumber::World)]
pub struct HarnessWorld {
    tools: Catalog,
    agents: Registry,
}

#[given(expr = "the tools catalog holds {string}")]
async fn tools_hold(w: &mut HarnessWorld, name: String) {
    w.tools.add(name);
}

#[given(expr = "the agents registry holds {string}")]
async fn agents_hold(w: &mut HarnessWorld, name: String) {
    w.agents.register(name);
}

#[then("the harness wiring succeeds")]
async fn wiring_succeeds(w: &mut HarnessWorld) {
    let agent = w.agents.get("main");
    let tool = w.tools.get("echo");
    ::core::assert!(agent.is_some());
    ::core::assert!(tool.is_some());
}

#[::tokio::main]
async fn main() {
    HarnessWorld::run("tests/features").await;
}
