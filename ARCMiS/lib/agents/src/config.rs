//! Typed view of `assets/configs/<run>/config.yml`. Every task-specific
//! value (model, paths, languages, toolchain) lives in the yml, not in the
//! agent. See .omp/rules/config.md.

/// Top-level config file shape.
#[derive(::core::fmt::Debug, ::serde::Deserialize, ::bon::Builder)]
pub struct Config {
    pub run: Run,
    pub output: Output,
    pub source: Source,
}

/// Run section: model identity and agent budget.
#[derive(::core::fmt::Debug, ::serde::Deserialize, ::bon::Builder)]
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
    /// think blocks on long prompts. Off keeps turns short and deterministic.
    #[serde(default = "default_think")]
    pub think: bool,
    /// Sampling temperature for the model.
    #[serde(default = "default_temperature")]
    pub temperature: f64,
}

/// Output section: where the transformed codebase and logs land.
#[derive(::core::fmt::Debug, ::serde::Deserialize, ::bon::Builder)]
pub struct Output {
    pub dir: ::std::path::PathBuf,
}

/// Source section: input codebase, target language, and toolchain.
#[derive(::core::fmt::Debug, ::serde::Deserialize, ::bon::Builder)]
pub struct Source {
    pub language: ::std::string::String,
    pub root: ::std::path::PathBuf,
    pub target: Target,
}

/// Target section: output language and test invocation.
#[derive(::core::fmt::Debug, ::serde::Deserialize, ::bon::Builder)]
pub struct Target {
    pub language: ::std::string::String,
    pub test_command: ::std::string::String,
}

const fn default_max_turns() -> usize {
    14
}

const fn default_num_ctx() -> u64 {
    16384
}

const fn default_max_output_tokens() -> u64 {
    8192
}

const fn default_max_retries() -> u32 {
    1
}

const fn default_think() -> bool {
    false
}

const fn default_temperature() -> f64 {
    0.2
}

impl Config {
    /// Load and parse the config file at `path`.
    pub fn load(path: &::std::path::Path) -> ::core::result::Result<Self, ::std::string::String> {
        let raw = ::std::fs::read_to_string(path)
            .map_err(|error| format!("config load failed for {}: {error}", path.display()))?;
        ::serde_yaml::from_str(&raw).map_err(|error| format!("config parse failed: {error}"))
    }
}
