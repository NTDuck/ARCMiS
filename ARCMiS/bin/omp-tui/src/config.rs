//! `config` carries every run knob. The binary resolves all values up front.

use crate::prompt::ORCHESTRATOR_PROMPT;
use anyhow::Context;
use std::path::PathBuf;

/// All settings the TUI needs. The binary reads no defaults inside library code.
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub omp_binary: String,
    pub omp_arguments: Vec<String>,
    pub working_dir: PathBuf,
    pub approval_mode: String,
    pub seed_prompt: String,
    pub pane_titles: [String; 5],
    pub log_file: PathBuf,
}

impl RunConfig {
    /// Build the config from CLI flags with repo-root defaults.
    pub fn from_args() -> anyhow::Result<Self> {
        let working_dir = std::env::args()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .canonicalize()
            .context("working directory")?;
        let omp_binary = std::env::var("OMP_BIN").unwrap_or_else(|_| "omp".to_string());
        let log_file = std::env::var("OMP_TUI_LOG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("omp-tui.log"));
        Ok(Self {
            omp_arguments: vec![
                "--mode".to_string(),
                "rpc".to_string(),
                format!("--append-system-prompt={ORCHESTRATOR_PROMPT}"),
            ],
            omp_binary,
            working_dir,
            approval_mode: "yolo".to_string(),
            seed_prompt: std::env::var("OMP_TUI_PROMPT").unwrap_or_default(),
            pane_titles: [
                "orchestrator".to_string(),
                "analyzer".to_string(),
                "planning".to_string(),
                "translator".to_string(),
                "validator".to_string(),
            ],
            log_file,
        })
    }
}
