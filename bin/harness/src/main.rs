//! Binary entry point for ARCMiS.

use ::arcmis_agents::Registry;
use ::arcmis_tools::Catalog;

#[tokio::main]
async fn main() -> ::std::process::ExitCode {
    // Placeholder wiring. Configuration bubbles up here per .omp/rules/config.md.
    let mut tools = Catalog::new();
    tools.add("echo");

    let mut agents = Registry::new();
    agents.register("main");

    match (agents.get("main"), tools.get("echo")) {
        (::core::option::Option::Some(_), ::core::option::Option::Some(_)) => {
            ::std::println!("arcmis harness ready: 1 agent, 1 tool");
            ::std::process::ExitCode::SUCCESS
        },
        _ => {
            ::std::eprintln!("arcmis harness failed to initialize");
            ::std::process::ExitCode::FAILURE
        },
    }
}
