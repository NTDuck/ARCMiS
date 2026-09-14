//! Typed view of `assets/configs/<run>/config.yml`. Every task-specific
//! value (model, paths, languages, toolchain) lives in the yml, not in the
//! agent. See .omp/rules/config.md.

use ::serde::Deserialize;

/// Top-level config file shape.
#[derive(::core::fmt::Debug, Deserialize)]
pub struct Config {
    pub run: Run,
    pub output: Output,
    pub source: Source,
}

/// Run section: model identity and agent budget.
#[derive(::core::fmt::Debug, Deserialize)]
pub struct Run {
    pub model: ::std::string::String,
    #[serde(default = "default_max_turns")]
    pub max_turns: usize,
    #[serde(default = "default_num_ctx")]
    pub num_ctx: u64,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Disable model thinking mode. Small models degenerate into runaway
    /// think blocks on long prompts; off keeps turns short and deterministic.
    #[serde(default = "default_think")]
    pub think: bool,
    /// Sampling temperature for the model.
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default)]
    pub source_files: ::std::vec::Vec<::std::string::String>,
    #[serde(default)]
    pub test_files: ::std::vec::Vec<::std::string::String>,
}

/// Output section: where the transformed codebase and logs land.
#[derive(::core::fmt::Debug, Deserialize)]
pub struct Output {
    pub dir: ::std::path::PathBuf,
    /// Package skeleton written before the agent runs. Paths are relative to
    /// the output dir. The agent overwrites these files freely.
    #[serde(default)]
    pub scaffold_files: ::std::collections::BTreeMap<::std::string::String, ::std::string::String>,
}

/// Source section: input codebase, target language, and toolchain.
#[derive(::core::fmt::Debug, Deserialize)]
pub struct Source {
    pub language: ::std::string::String,
    pub root: ::std::path::PathBuf,
    pub target: Target,
}

/// Target section: output language, test invocation, and style guidance.
#[derive(::core::fmt::Debug, Deserialize)]
pub struct Target {
    pub language: ::std::string::String,
    pub test_command: ::std::string::String,
    /// One-line style hint appended to the prompt (target-idiom guidance).
    #[serde(default)]
    pub style_hint: ::std::string::String,
    /// File names the agent may not write (toolchain-owned).
    #[serde(default = "default_protected_files")]
    pub protected_files: ::std::vec::Vec<::std::string::String>,
}

const fn default_max_turns() -> usize {
    12
}

const fn default_num_ctx() -> u64 {
    8192
}

const fn default_max_output_tokens() -> u64 {
    8192
}

const fn default_max_retries() -> u32 {
    2
}

const fn default_think() -> bool {
    false
}

const fn default_temperature() -> f64 {
    0.2
}

fn default_protected_files() -> ::std::vec::Vec<::std::string::String> {
    ::std::vec![::std::string::String::from("Cargo.toml"), ::std::string::String::from("Cargo.lock"),]
}

impl Config {
    /// Load and parse the config file at `path`.
    pub fn load(path: &::std::path::Path) -> ::core::result::Result<Self, ::std::string::String> {
        let raw =
            ::std::fs::read_to_string(path).map_err(|e| format!("config load failed for {}: {e}", path.display()))?;
        ::serde_yaml::from_str(&raw).map_err(|e| format!("config parse failed: {e}"))
    }
}
