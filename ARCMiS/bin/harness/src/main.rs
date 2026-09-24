//! Binary entry point for ARCMiS. Thin stub for the v1 rebuild: load the
//! config, print the banner, exit. The MAS harness driver replaces this in a
//! later phase.

use std::process::ExitCode;

use agents::Config;
use anyhow::Result;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("harness failed: {error:#}");
            ExitCode::FAILURE
        },
    }
}

/// Load the config from `argv[1]` (default
/// `assets/configs/GildedRose-Refactoring-Kata/config.yml`).
fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let config_path =
        args.get(1).cloned().unwrap_or_else(|| "assets/configs/GildedRose-Refactoring-Kata/config.yml".to_owned());
    let _config = Config::load(std::path::Path::new(&config_path))?;
    println!("ARCMiS MAS harness v1");
    Ok(())
}
