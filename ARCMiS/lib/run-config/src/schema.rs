//! Top-level run configuration sections.
//!
//! One struct per YAML section. Fields without a genuine default are
//! mandatory in the file. Optional sections use `#[serde(default)]` with
//! an inline `Default` impl, per `.omp/rules/config.md`.

use serde::Deserialize;

/// Top-level run configuration. Deserialized straight from the mandatory
/// run config file. Mandatory-field omissions are rejected at preflight
/// (see `validation`): a quiet `""` default would hide the run's real
/// inputs (ADR 0032).
#[derive(Debug, Deserialize)]
pub struct SectionConfig {
    /// Identifies the run in logs and output artifacts. Must not be empty.
    pub run: RunName,
    /// Input and output locations. Both are mandatory.
    pub paths: super::PathsConfig,
    /// Source and target languages, plus the target test command.
    #[serde(default)]
    pub languages: LanguagesConfig,
    /// Model selection. Feeds the launch arguments built by `omprpc`.
    #[serde(default)]
    pub model: ModelConfig,
    /// Static hierarchy layout, one entry per lead team (ADR 0026).
    #[serde(default)]
    pub hierarchy: super::HierarchyConfig,
    /// Shared blackboard coordination settings.
    #[serde(default)]
    pub blackboard: BlackboardConfig,
    /// Tool gateway policy. Allowlist entries beat denylist entries.
    #[serde(default)]
    pub guard: GuardConfig,
    /// Context-compaction thresholds, one pair for every agent.
    #[serde(default)]
    pub compaction: CompactionConfig,
    /// Typed-decision guard settings. Local checkpoint only (ADR 0023).
    #[serde(default)]
    pub laya: LayaConfig,
    /// Budget ceilings that bound the run loop.
    #[serde(default)]
    pub limits: LimitsConfig,
}

/// Run identification.
#[derive(Debug, Deserialize)]
pub struct RunName {
    /// Human-readable run identifier, cited in logs and artifacts.
    pub name: String,
}

/// Source and target programming-language names, for example `java` and
/// `rust`. Kept as free-form strings: the schema must not hold them tighter
/// than the verifying tools do (`.omp/rules/no-tuning.md`).
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct LanguagesConfig {
    /// One `{ source, target }` language pair per run. ReCodeAgent runs
    /// one pair per run over its tool-project datasets.
    pub pair: LanguagePair,
    /// Raw target-language test command, invoked by the harness during
    /// validation and migration phases.
    pub test_command: String,
}

impl Default for LanguagesConfig {
    fn default() -> Self {
        Self {
            pair: LanguagePair {
                source: String::new(),
                target: String::new(),
            },
            test_command: String::new(),
        }
    }
}

/// One `{ source, target }` programming-language pair.
#[derive(Debug, Deserialize)]
pub struct LanguagePair {
    /// Programming language of `paths.input_dir`.
    pub source: String,
    /// Programming language produced into `paths.output_dir`.
    pub target: String,
}

/// Model selection for the run. The identifiers feed the `omp` launch
/// arguments assembled by the `omprpc` crate. This section stays plain
/// strings: argument shaping belongs to the transport, not the schema.
#[derive(Debug, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct ModelConfig {
    /// Optional model identifier overriding the default SLM. Absent keeps
    /// the launching side's default posture (frontier fallback excluded:
    /// see PRODUCT-SPECS.md).
    pub id: Option<String>,
    /// Extra arguments appended verbatim to the `omp` launch command.
    pub extra_args: Vec<String>,
}

/// Shared blackboard coordination settings. All agent coordination flows
/// through topics: findings, plans, and artifacts cite the blackboard as
/// the single rendezvous point (ADR 0031).
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct BlackboardConfig {
    /// Topic revisions kept per topic. Older revisions roll off when a
    /// writer exceeds this bound. Zero keeps every revision.
    pub max_topic_revisions: usize,
    /// Direct messages per agent mailbox. Oldest messages drop first.
    pub inbox_depth: usize,
}

impl Default for BlackboardConfig {
    fn default() -> Self {
        Self {
            max_topic_revisions: 16,
            inbox_depth: 32,
        }
    }
}

/// Tool gateway policy. Every tool call passes the gateway before it
/// executes. Deny rules win over allow rules: the safe answer to an
/// ambiguous call is refusal with feedback (ADR 0031).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct GuardConfig {
    /// Always-permitted tool names.
    pub allowlist: Vec<String>,
    /// Always-denied tool names and path prefixes.
    pub denylist: Vec<String>,
}

/// Context-compaction thresholds for every agent. When an agent's projected
/// input crosses `threshold_tokens`, the oldest context archives down to
/// `keep_recent_tokens`. Zero disables compaction for all agents.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct CompactionConfig {
    /// Compaction trigger point, in projected input tokens.
    pub threshold_tokens: u32,
    /// Recent tokens compaction keeps verbatim.
    pub keep_recent_tokens: u32,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            threshold_tokens: 12000,
            keep_recent_tokens: 4000,
        }
    }
}

/// Laya typed-decision guard settings (ADRs 0023, 0027). Local checkpoint
/// only: the hub feature stays off, so sandboxed runs never fetch models.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct LayaConfig {
    /// Master switch. Off disables every typed-decision consultation.
    pub enabled: bool,
    /// Local checkpoint directory. Empty with `enabled` reports a
    /// validation finding.
    pub checkpoint_dir: String,
    /// Minimum top-label confidence accepting a typed decision. At or
    /// below the threshold the decision cascades back to the caller's
    /// unmodified behavior (arXiv:2609.26550 §7).
    pub confidence_threshold: f64,
}

impl Default for LayaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            checkpoint_dir: String::new(),
            confidence_threshold: 0.9,
        }
    }
}

/// Budget ceilings bounding the run loop.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct LimitsConfig {
    /// Total orchestrator round budget per run.
    pub turns: usize,
    /// Maximum agents dispatched concurrently per round. Bounded by
    /// single-model engine slots today (ADR 0022).
    pub fanout: usize,
    /// Inner-loop round ceiling shared by all lead teams (ADR 0026).
    pub collective_turns: usize,
    /// Repair attempts per diagnosed failure before escalation.
    pub max_repairs: usize,
    /// Consecutive stagnant rounds before the loop escalates.
    pub stagnation_rounds: usize,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            turns: 60,
            fanout: 2,
            collective_turns: 40,
            max_repairs: 2,
            stagnation_rounds: 3,
        }
    }
}
