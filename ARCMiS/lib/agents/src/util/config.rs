//! Typed view of `assets/configs/<run>/config.yml`. Every task-specific
//! value (model, paths, languages, toolchain) lives in the yml, not in the
//! agent. See .omp/rules/config.md.

use std::collections::BTreeMap;
use std::fs::read_to_string;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use serde::Deserialize;

/// Top-level config file shape.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub run: Run,
    pub output: Output,
    pub source: Source,
    /// MAS method configuration. Ignored by the other methods.
    #[serde(default)]
    pub mas: MasConfig,
    /// Snapcompact archival policy for the agents (PNG-frame compaction).
    #[serde(default)]
    pub snapcompact: SnapcompactConfig,
}

/// Snapcompact section: when an agent's projected input crosses the token
/// threshold, the hook compacts the discarded history into PNG frames and
/// attaches them to the next model call.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(default)]
pub struct SnapcompactConfig {
    /// Projection threshold that triggers compaction. 0 disables the hook.
    pub threshold_tokens: u32,
    /// Recent tokens kept verbatim. the rest is archived into frames.
    pub keep_recent_tokens: u32,
}

impl Default for SnapcompactConfig {
    fn default() -> Self {
        Self {
            threshold_tokens: 12000,
            keep_recent_tokens: 4000,
        }
    }
}

/// Run section: model identity and agent budget.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Run {
    pub model: String,
    pub max_turns: usize,
    pub num_ctx: u64,
    pub max_output_tokens: u64,
    pub max_retries: u32,
    /// Enable model thinking mode. Thinking improves long-horizon tool
    /// use on the supported models. disable only for a specific reason.
    pub think: bool,
    /// Sampling temperature for the model.
    pub temperature: f64,
    /// Provider name: `ollama` (default) or `netmind`.
    pub provider: String,
    /// Per-request timeout in seconds on the model-call HTTP client. 0
    /// (default) means no timeout, the pre-knob behavior. A timed-out
    /// request counts as transient and routes through the existing
    /// retry/backoff path (ADR 0028: bounded silent inference turns).
    pub request_timeout_secs: u64,
}
impl Default for Run {
    fn default() -> Self {
        Self {
            model: String::new(),
            max_turns: 14,
            num_ctx: 16384,
            max_output_tokens: 8192,
            max_retries: 1,
            think: true,
            temperature: 0.2,
            provider: "ollama".to_owned(),
            request_timeout_secs: 0,
        }
    }
}

impl Run {
    /// Whether the provider is the local ollama daemon (native protocol,
    /// context-window and think params ride along).
    #[must_use]
    pub fn provider_is_ollama(&self) -> bool {
        self.provider == "ollama"
    }
}

/// MAS method configuration: budgets, fleet ladder, and specialist caps.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MasConfig {
    /// Orchestrator-loop round ceiling across the whole run.
    pub max_rounds: usize,
    /// Orchestrator turn budget per round.
    pub orchestrator_turns: usize,
    /// Specialist turn budget per delegation.
    pub worker_turns: usize,
    /// Turn budget for the judge roles (validator, critic, fleet analyst):
    /// they read, run the suite, and write one verdict. a small ceiling keeps
    /// a validation pass from consuming a translator-sized budget.
    pub judge_turns: usize,
    /// Repair attempts per diagnosed failure before escalation.
    pub max_repairs: usize,
    /// Consecutive stalled rounds before the orchestrator escalates.
    pub stagnation_rounds: usize,
    /// plan.md character cap (GVS5H keeps the plan inside one read).
    pub plan_cap: usize,
    /// notes.md character cap.
    pub notes_cap: usize,
    /// Model ladder, weakest first. The fleet analyst promotes along it.
    pub model_ladder: Vec<String>,
    /// Test generations the tester may write per module (coverage-gap cap).
    pub max_generated_tests_per_module: usize,
    /// Parallel specialist executions the orchestrator may interleave in
    /// one round (bounded by the single ollama slot. 2 = two turn streams).
    pub fanout: usize,
    /// Model context window, used by the guard's read-scoping gate (0
    /// disables the gate). Mirrors `run.num_ctx`. the orchestrator cannot
    /// see the run section.
    pub num_ctx: u64,
    /// Per-role output-token overrides over `run.max_output_tokens`.
    /// Translator file-emission turns die at the run default (c5 nandc
    /// escalation). a role whose deliverable is one large file needs its
    /// own ceiling. A role missing from the map keeps the run default.
    pub role_output_tokens: BTreeMap<String, u64>,
    /// Per-role think overrides over `run.think`. A role missing from
    /// the map keeps the run default (c8 fileupload: translator
    /// reasoning filled the window before its first write).
    pub role_think: BTreeMap<String, bool>,
    /// Guard policy: deterministic deny patterns and the model-arbitration
    /// switch (the Jev slot. off until a second model exists).
    pub guard: GuardConfig,
    /// Hierarchical static orchestration (ADR 0026). Default off: the run
    /// behaves exactly as the single-tier ADR 0022 design.
    pub hierarchy: HierarchyConfig,
    /// Laya-backed typed triage on the lead's member dispatches (ADR 0027).
    /// Off by default: without a section the lead loop is uninstrumented.
    pub jev_triage: JevTriageConfig,
}

/// Tier-2 team layout for hierarchical orchestration. The build step fixes
/// the teams (registry, one lead prompt, one member allowlist each). The
/// runtime picks which lead serves a delegation. No team layout is
/// hardcoded here: the config owns the split.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(default)]
pub struct HierarchyConfig {
    /// Tier-2 leads active. false = exact single-tier behavior.
    pub enabled: bool,
    /// Deny a tier-1 delegation that names a specialist directly (feedback
    /// to the orchestrator) instead of silently rerouting it to the
    /// member's lead. On by default: the refusal teaches the tier split.
    pub deny_direct: bool,
    /// One entry per lead. The lead's prompt file must exist. members must
    /// be specialist role names.
    pub teams: Vec<TeamConfig>,
}

impl Default for HierarchyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            deny_direct: true,
            teams: Vec::new(),
        }
    }
}

/// One tier-2 team: a lead and the specialists it may dispatch.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct TeamConfig {
    /// Lead role name (for example `migration-lead`). Resolves to a prompt
    /// file. the agent itself is tool-free and decides in DECISION text.
    pub lead: String,
    /// Specialist role names this lead may delegate to. Cross-team and
    /// lead-to-lead delegation is refused (depth cap 3, ADR 0026).
    pub members: Vec<String>,
    /// Inner-loop round ceiling for the lead (its own model calls).
    pub turns: usize,
    /// Consecutive unproductive inner rounds before the lead's loop
    /// breaks. Empty keeps the run-level `stagnation_rounds`.
    pub stagnation_rounds: Option<usize>,
}

impl TeamConfig {
    /// Resolve the member names to roles. A name that fails to resolve is
    /// a config error. `validate_teams` reports it at preflight.
    #[must_use]
    pub fn members(&self) -> Vec<Option<crate::mas::roles::Role>> {
        self.members.iter().map(|name| crate::mas::roles::Role::from_name(name)).collect()
    }
}

/// Deterministic + model-arbitrated tool gateway policy.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(default)]
pub struct GuardConfig {
    /// Bash command substrings that are always denied.
    pub deny: Vec<String>,
    /// Tool calls allowed per delegation before the guard denies further
    /// calls. 0 disables the cap. Judges (validator, critic) promise a
    /// small call budget in their prompts. the guard enforces it because
    /// the model alone will loop otherwise.
    pub max_tool_calls: usize,
    /// Laya-backed Jev judge over the guard's Ask slot (ADR 0023). Off by
    /// default: without a section the guard behaves exactly as before.
    pub jev_judge: JevJudgeConfig,
}

/// Configuration for the laya-backed Jev judge (ADR 0023). One typed
/// decision question per consultation. the confidence threshold gates
/// acceptance per the cascade rule of arXiv:2609.26550 §7.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(default)]
pub struct JevJudgeConfig {
    /// Consult the judge on Ask-class calls when true.
    pub enabled: bool,
    /// Local laya checkpoint directory. No hub download happens: an empty
    /// path with `enabled` keeps the judge off.
    pub checkpoint: String,
    /// Minimum top-label probability that accepts a verdict. Below it the
    /// caller falls back to its previous behavior (arXiv:2609.26550 §7:
    /// accept when confident, escalate when unsure).
    pub confidence_threshold: f64,
}

impl Default for JevJudgeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            checkpoint: String::new(),
            confidence_threshold: 0.9,
        }
    }
}

/// What a triage verdict does (ADR 0028). `observe` keeps verdicts as
/// ledger rows. `enforce` ends the round or lead batch early on a
/// confident stop-or-harmful verdict. Default `observe`: zero behavior
/// change unless configured.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TriagePolicy {
    /// Ledger only. Behavior is unchanged.
    #[default]
    Observe,
    /// A confident stop-or-harmful verdict ends the round or batch early.
    Enforce,
}

/// Configuration for the laya-backed typed triage on the lead's member
/// dispatches (ADR 0027). One `agent_trace_observability` predict call per
/// dispatch result. The confidence threshold gates acceptance per the
/// cascade rule of arXiv:2609.26550 §7: accept when confident, fall back
/// when unsure.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(default)]
pub struct JevTriageConfig {
    /// Consult the triage judge after each member dispatch when true.
    pub enabled: bool,
    /// Local laya checkpoint directory (the typed-decisions checkpoint).
    /// No hub download happens: an empty path with `enabled` keeps the
    /// judge off.
    pub checkpoint: String,
    /// Minimum top-label probability across all five answers that accepts a
    /// verdict. Below it the lead loop falls back to uninstrumented
    /// behavior (arXiv:2609.26550 §7).
    pub confidence_threshold: f64,
    /// What a verdict does. `observe` (default) keeps it a ledger row.
    /// `enforce` ends the round or lead batch early on a confident
    /// stop-or-harmful verdict. See ADR 0028.
    pub policy: TriagePolicy,
}

impl Default for JevTriageConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            checkpoint: String::new(),
            confidence_threshold: 0.9,
            policy: TriagePolicy::default(),
        }
    }
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            deny: vec![
                "rm -rf".into(),
                "git push".into(),
                "git reset --hard".into(),
                "curl ".into(),
                "wget ".into(),
                "pip install".into(),
                "cargo install".into(),
                "npm install".into(),
                "sudo ".into(),
            ],
            max_tool_calls: 0,
            jev_judge: JevJudgeConfig::default(),
        }
    }
}
impl Default for MasConfig {
    fn default() -> Self {
        Self {
            max_rounds: 60,
            orchestrator_turns: 20,
            worker_turns: 40,
            judge_turns: 40,
            max_repairs: 2,
            stagnation_rounds: 3,
            plan_cap: 4000,
            notes_cap: 8000,
            model_ladder: Vec::new(),
            max_generated_tests_per_module: 4,
            fanout: 2,
            num_ctx: 32768,
            role_output_tokens: BTreeMap::new(),
            role_think: BTreeMap::new(),
            guard: GuardConfig::default(),
            hierarchy: HierarchyConfig::default(),
            jev_triage: JevTriageConfig::default(),
        }
    }
}

/// Output section: where the transformed codebase and logs land.
#[derive(Debug, Deserialize)]
pub struct Output {
    pub dir: PathBuf,
}

/// Source section: input codebase, target language, and toolchain.
#[derive(Debug, Deserialize)]
pub struct Source {
    pub language: String,
    pub root: PathBuf,
    pub target: Target,
}

/// Target section: output language and test invocation.
#[derive(Debug, Deserialize)]
pub struct Target {
    pub language: String,
    pub test_command: String,
}

impl Config {
    /// Load and parse the config file at `path`.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = read_to_string(path).with_context(|| format!("config load failed for {}", path.display()))?;
        serde_yaml::from_str(&raw).with_context(|| format!("config parse failed for {}", path.display()))
    }
}

#[cfg(test)]
mod policy_tests {
    use super::JevTriageConfig;
    use super::Run;
    use super::TriagePolicy;

    #[test]
    fn defaults_are_observe_and_no_timeout() {
        let config = JevTriageConfig::default();
        assert_eq!(config.policy, TriagePolicy::Observe);
        assert!(!config.enabled);
        assert_eq!(Run::default().request_timeout_secs, 0);
    }

    #[test]
    fn yml_parse_accepts_the_new_knobs() {
        let run: Run = serde_yaml::from_str("model: m\nmax_turns: 14\nnum_ctx: 16384\nmax_output_tokens: 8192\nmax_retries: 1\nthink: true\ntemperature: 0.2\nprovider: ollama\nrequest_timeout_secs: 900\n").expect("run");
        assert_eq!(run.request_timeout_secs, 900);
        let jev: JevTriageConfig = serde_yaml::from_str("enabled: true\ncheckpoint: assets/models/laya-typed-decisions\nconfidence_threshold: 0.9\npolicy: enforce\n").expect("jev");
        assert_eq!(jev.policy, TriagePolicy::Enforce);
        assert_eq!(jev.confidence_threshold, 0.9);
    }

    #[test]
    fn yml_without_the_knobs_keeps_defaults() {
        let run: Run = serde_yaml::from_str("model: m\nmax_turns: 14\nnum_ctx: 16384\nmax_output_tokens: 8192\nmax_retries: 1\nthink: true\ntemperature: 0.2\nprovider: ollama\n").expect("run");
        assert_eq!(run.request_timeout_secs, 0);
        let jev: JevTriageConfig =
            serde_yaml::from_str("enabled: true\ncheckpoint: x\nconfidence_threshold: 0.9\n").expect("jev");
        assert_eq!(jev.policy, TriagePolicy::Observe);
    }

    #[test]
    fn full_config_file_roundtrips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.yml");
        std::fs::write(
            &path,
            "run:\n  model: m\n  max_turns: 14\n  num_ctx: 16384\n  max_output_tokens: 8192\n  max_retries: 1\n  think: true\n  temperature: 0.2\n  provider: ollama\n  request_timeout_secs: 300\noutput:\n  dir: /tmp/x\nsource:\n  language: go\n  root: src\n  target:\n    language: rust\n    test_command: cargo test\n",
        )
        .expect("write config");
        let config = super::Config::load(&path).expect("load");
        assert_eq!(config.run.request_timeout_secs, 300);
        assert_eq!(config.mas.jev_triage.policy, TriagePolicy::Observe);
    }
}
