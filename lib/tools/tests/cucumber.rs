//! Cucumber suite for the tool catalog.

use ::arcmis_tools::{Catalog, Tool};
use ::cucumber::{given, then, when, World};

#[derive(::core::fmt::Debug, ::core::default::Default)]
#[derive(World)]
pub struct ToolWorld {
    catalog: Catalog,
    last: ::core::option::Option<Tool>,
}

#[given("an empty catalog")]
async fn empty_catalog(w: &mut ToolWorld) {
    w.catalog = Catalog::new();
}

#[when(expr = "I add the tool {string}")]
async fn add_tool(w: &mut ToolWorld, name: String) {
    w.catalog.add(name);
}

#[then(expr = "the catalog holds {int} tool")]
async fn holds_tools(w: &mut ToolWorld, expected: usize) {
    ::core::assert_eq!(w.catalog.len(), expected);
}

#[then(expr = "the tool {string} is present")]
async fn tool_present(w: &mut ToolWorld, name: String) {
    let tool = w.catalog.get(&name);
    ::core::assert!(tool.is_some());
    let tool = tool.expect("checked above");
    w.last = ::core::option::Option::Some(::arcmis_tools::Tool {
        name: ::std::clone::Clone::clone(&tool.name),
    });
}

#[then(expr = "the tool {string} is absent")]
async fn tool_absent(w: &mut ToolWorld, name: String) {
    ::core::assert!(w.catalog.get(&name).is_none());
}

#[tokio::main]
async fn main() {
    ToolWorld::run("tests/features").await;
}
