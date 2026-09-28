//! Jev-as-a-guard: the tool gateway as an `on_tool_call` hook. Every call
//! from every agent passes the deterministic tier first — role allowlist,
//! path policy (`source/` read-only), deny patterns for bash. Rejections
//! return `ToolCallAction::Skip(feedback)` so the model sees the reason and
//! can self-correct; the breaker counts repeats. Denied Ask-class calls
//! (the `ask` tool) may arbitrate through the laya-backed Jev judge
//! (ADR 0023); every other denial, and budget exhaustion, is final.
//! Without an enabled judge the Ask slot stays deny-by-default.

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;

use agents::util::config::GuardConfig;
use agents::Role;
use rig::agent::hook::AgentHook;
use rig::agent::hook::CompletionResponse;
use rig::agent::hook::HookContext;
use rig::agent::hook::ObservationAction;
use rig::agent::hook::ToolCall;
use rig::agent::hook::ToolCallAction;
use serde_json::Value;

use crate::guard::Guard;
use crate::jev_judge::Consultation;
use crate::jev_judge::JevJudge;

/// The tool gateway over one role. Built per delegation; the registry does
/// not attach it because the role is only known at dispatch time.
#[derive(Clone)]
pub struct GuardHook {
    inner: Guard,
    /// This delegation's role; the judge state names it.
    role: Role,
    policy: GuardConfig,
    /// The laya judge over the Ask slot; `None`-equivalent state keeps the
    /// deny-by-default behavior (ADR 0023).
    judge: JevJudge,
    /// Consecutive rejections inside this delegation; the manager sees the
    /// count when the delegation ends.
    rejections: Arc<AtomicUsize>,
    /// Last rejection feedback (for the delegation report).
    last_rejection: Arc<Mutex<Option<String>>>,
    /// Tool calls executed so far in this delegation; the policy cap denies
    /// further calls once this crosses `max_tool_calls`.
    calls: Arc<AtomicUsize>,
    /// Model context window for this run (0 disables read-scoping).
    num_ctx: u64,
    /// Projected next-turn input tokens: last observed prompt usage plus
    /// this delegation's unread file bytes. The read gate compares a
    /// candidate read against this projection.
    projected_tokens: Arc<AtomicU64>,
}

impl GuardHook {
    /// Build the gateway for one role under the run policy. `num_ctx` is
    /// the model's context window; 0 disables read-scoping.
    #[must_use]
    pub fn new(role: Role, workspace: blackboard::Workspace, policy: GuardConfig, num_ctx: u64) -> Self {
        let judge = JevJudge::from_config(&policy.jev_judge);
        Self {
            inner: Guard::new(role.allowed_tools().to_vec(), workspace),
            role,
            policy,
            judge,
            rejections: Arc::new(AtomicUsize::new(0)),
            last_rejection: Arc::new(Mutex::new(None)),
            calls: Arc::new(AtomicUsize::new(0)),
            num_ctx,
            projected_tokens: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Rejections so far in this delegation.
    #[must_use]
    pub fn rejection_count(&self) -> usize {
        self.rejections.load(Ordering::Relaxed)
    }

    /// The last rejection feedback, if any.
    #[must_use]
    pub fn last_rejection(&self) -> Option<String> {
        self.last_rejection.lock().map(|guard| guard.clone()).unwrap_or_default()
    }

    /// Deterministic verdict for one call. `None` means allow. Tiers, in
    /// order: allowlist, path policy, deny patterns.
    fn deterministic(&self, tool_name: &str, args: &str) -> Option<String> {
        if !self.inner.permits_tool(tool_name) {
            return Some(format!(
                "tool '{tool_name}' is outside this role's allowlist; use one of: {}",
                self.inner.allowed().join(", ")
            ));
        }
        if let Some(path) = arg_path_str(args) {
            // The source/ readonly rule constrains mutations only. Applying
            // it to read-class tools makes `source/` unreadable by the very
            // roles whose job is to analyze and translate it (baseline
            // 20260926T211000Z: every analyst read of source/ was skipped).
            if is_write_tool(tool_name) {
                if let Err(error) = self.inner.permits_path(tool_name, &PathBuf::from(&path)) {
                    return Some(error.to_string());
                }
            }
            // Read-scoping: a read that would push the projected next-turn
            // input past the context window is skipped with a scope hint
            // (c4 evidence: 30k+ saturating turns died as Length with no
            // answer). Applies to read-class tools only. Tool paths resolve
            // from the workspace root, so size checks must too; a harness-
            // cwd lookup misses and silently disables the gate.
            if tool_name == "read" && self.num_ctx > 0 {
                if let Some(size) = self.file_size_bytes(&path) {
                    let projected = self.projected_tokens.load(Ordering::Relaxed);
                    // ~4 bytes per token, plus headroom for tool-result JSON
                    // framing.
                    let file_tokens = (size / 4).saturating_add(64);
                    if projected + file_tokens > self.num_ctx * 8 / 10 {
                        return Some(self.scope_feedback());
                    }
                }
            }
        }
        if tool_name == "bash" {
            if let Some(command) = command_str(args) {
                for pattern in &self.policy.deny {
                    if command.contains(pattern.as_str()) {
                        return Some(format!("command denied by policy: contains '{pattern}'"));
                    }
                }
            }
        }
        None
    }
}

impl AgentHook for GuardHook {
    async fn on_completion_response(&self, _ctx: &HookContext, event: CompletionResponse<'_>) -> ObservationAction {
        // Project the next turn's input from the last observed prompt usage.
        // Tool results land between turns, so the read gate adds candidate
        // file tokens on top of this floor.
        self.projected_tokens.store(event.usage.input_tokens.max(event.usage.total_tokens), Ordering::Relaxed);
        ObservationAction::Continue
    }

    async fn on_tool_call(&self, _ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        self.gate_call(event.tool_name, event.args)
    }
}

impl GuardHook {
    /// The tool gateway: deterministic tier first, then budget. A denied
    /// Ask-class call may arbitrate through the Jev judge (ADR 0023).
    /// Budget exhaustion and every non-ask denial are never arbitrable.
    fn gate_call(&self, tool_name: &str, args: &str) -> ToolCallAction {
        let verdict = self.deterministic(tool_name, args).or_else(|| self.budget_verdict());
        if let Some(reason) = verdict {
            // The Jev slot (ADR 0023): a confident judge verdict can allow
            // a denied Ask-class call. Budget exhaustion and every other
            // denial are never arbitrable. Fallback keeps the deny.
            if tool_name == "ask" && self.arbitrate_ask(tool_name, args) {
                return ToolCallAction::Run;
            }
            self.rejections.fetch_add(1, Ordering::Relaxed);
            if let Ok(mut slot) = self.last_rejection.lock() {
                *slot = Some(format!("{tool_name}: {reason}"));
            }
            return ToolCallAction::Skip(format!(
                "GUARD REFUSAL: {reason}. Stay inside the role's tools and the workspace layout."
            ));
        }
        ToolCallAction::Run
    }

    /// Consult the Jev judge on a denied Ask-class call. True when the
    /// judge is confident the call fits the role. Any fallback (disabled
    /// judge, load or inference error, low confidence, foreign labels)
    /// keeps the deterministic denial (arXiv:2609.26550 §7: unsure always
    /// defers). Non-ask tools never arbitrate.
    fn arbitrate_ask(&self, tool_name: &str, args: &str) -> bool {
        if !self.judge.is_enabled() {
            return false;
        }
        let args = serde_json::from_str::<Value>(args).unwrap_or(Value::Null);
        match self.judge.consult_ask(self.role.name(), tool_name, &args) {
            Consultation::Decided {
                verdict: crate::jev_judge::AskVerdict::Appropriate,
                confidence,
            } => {
                tracing::info!(tool = tool_name, confidence, "jev judge allowed a denied call");
                true
            },
            _ => false,
        }
    }

    /// Budget verdict for the next tool call. `None` means allow (and
    /// counts the call); `Some` denies with the budget-exhausted reason.
    fn budget_verdict(&self) -> Option<String> {
        let cap = self.policy.max_tool_calls;
        if cap > 0 && self.calls.load(Ordering::Relaxed) >= cap {
            Some(format!("tool-call budget exhausted ({cap} calls used); write the final verdict now"))
        } else {
            self.calls.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// The scope hint returned when a read would saturate the window.
    fn scope_feedback(&self) -> String {
        "read would exhaust the context window; scope to one module; read head and grep symbols; summarize as you go"
            .into()
    }

    /// Size of an existing file in bytes, resolved from the workspace root
    /// exactly as tool paths resolve. `None` when the path is not a plain
    /// file (directories and virtual paths skip the gate).
    fn file_size_bytes(&self, path: &str) -> Option<u64> {
        let resolved = self.inner.workspace().root().join(path);
        std::fs::metadata(resolved).ok().filter(|meta| meta.is_file()).map(|meta| meta.len())
    }
}

/// Extract the write target from a raw JSON args string. Tools without a
/// path-shaped argument return `None`.
fn arg_path_str(args: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(args).ok()?;
    value.get("path").or_else(|| value.get("file")).and_then(serde_json::Value::as_str).map(str::to_owned)
}

/// Extract the command from a bash tool's args.
fn command_str(args: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(args).ok()?;
    value.get("command").and_then(serde_json::Value::as_str).map(str::to_owned)
}

/// Whether the tool can mutate the workspace. Only these meet the source/
/// readonly rule; read-class tools pass the path check untouched.
fn is_write_tool(tool_name: &str) -> bool {
    matches!(tool_name, "write" | "edit" | "ast_edit")
}

#[cfg(test)]
mod tests {
    use blackboard::Workspace;

    use super::*;

    fn workspace() -> Workspace {
        let dir = std::env::temp_dir().join(format!("guard-hook-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(dir.join("target"));
        let _ = std::fs::create_dir_all(dir.join("source"));
        Workspace::new(dir)
    }

    fn hook(role: Role) -> GuardHook {
        GuardHook::new(role, workspace(), GuardConfig::default(), 32768)
    }

    #[test]
    fn denies_tool_outside_role_allowlist() {
        let hook = hook(Role::Validator);
        let verdict = hook.deterministic("write", r#"{"path":"target/x.rs","content":"fn main() {}"}"#);
        assert!(verdict.is_some());
        assert!(verdict.unwrap().contains("allowlist"));
    }

    #[test]
    fn denies_write_into_source() {
        let hook = hook(Role::Translator);
        let verdict = hook.deterministic("write", r#"{"path":"source/main.c","content":"int x;"}"#);
        assert!(verdict.is_some());
    }

    #[test]
    fn denies_bash_deny_pattern() {
        let hook = hook(Role::Translator);
        let verdict = hook.deterministic("bash", r#"{"command":"rm -rf /"}"#);
        assert!(verdict.is_some());
        assert!(verdict.unwrap().contains("denied by policy"));
    }

    #[test]
    fn disabled_judge_keeps_ask_slot_deny_by_default() {
        // Default config: no judge. A denied call must still deny, and no
        // arbitration may fire.
        let hook = hook(Role::Validator);
        assert!(!hook.judge.is_enabled());
        assert!(!hook.arbitrate_ask("write", r#"{"path":"target/x.rs"}"#));
    }

    #[test]
    fn enabled_judge_with_bad_checkpoint_denies() {
        let policy = GuardConfig {
            jev_judge: agents::util::config::JevJudgeConfig {
                enabled: true,
                checkpoint: "/nonexistent/jev-checkpoint".into(),
                confidence_threshold: 0.9,
            },
            ..GuardConfig::default()
        };
        let hook = GuardHook::new(Role::Validator, workspace(), policy, 32768);
        assert!(!hook.judge.is_enabled());
        assert!(!hook.arbitrate_ask("write", r#"{"path":"target/x.rs"}"#));
    }

    /// A hook with an enabled (though fallback-prone) judge section.
    fn hook_with_judge() -> GuardHook {
        let policy = GuardConfig {
            jev_judge: agents::util::config::JevJudgeConfig {
                enabled: true,
                checkpoint: "/nonexistent/jev-checkpoint".into(),
                confidence_threshold: 0.9,
            },
            ..GuardConfig::default()
        };
        GuardHook::new(Role::Validator, workspace(), policy, 32768)
    }

    #[test]
    fn budget_exhaustion_is_never_arbitrable() {
        let hook = hook_with_judge();
        let mut action = hook.gate_call("write", r#"{"path":"target/x.rs","content":"fn f() {}"}"#);
        for _ in 1..3 {
            action = hook.gate_call("write", r#"{"path":"target/x.rs","content":"fn f() {}"}"#);
        }
        // Third call hits the budget; the enabled judge must not allow it.
        assert!(matches!(action, ToolCallAction::Skip(_)), "budget denial must stand: {action:?}");
    }

    #[test]
    fn bash_deny_pattern_is_never_arbitrable() {
        let hook = hook_with_judge();
        let action = hook.gate_call("bash", r#"{"command":"rm -rf /"}"#);
        assert!(matches!(action, ToolCallAction::Skip(_)), "deny-pattern denial must stand: {action:?}");
    }

    #[test]
    fn confident_ask_verdict_allows_the_call() {
        // The committed tiny fixture is random-init, so q sits near 0.5;
        // tau 0.45 admits it. `ask` is outside the Validator allowlist, so
        // the deterministic tier denies and the judge arbitrates.
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.omp/skills/laya/references/tests/fixtures/tiny");
        let policy = GuardConfig {
            jev_judge: agents::util::config::JevJudgeConfig {
                enabled: true,
                checkpoint: fixture.to_string_lossy().into_owned(),
                confidence_threshold: 0.45,
            },
            ..GuardConfig::default()
        };
        let hook = GuardHook::new(Role::Validator, workspace(), policy, 32768);
        assert!(hook.judge.is_enabled());
        let action = hook.gate_call(
            "ask",
            r#"{"questions":[{"id":"q1","question":"Continue?","options":[{"label":"yes"},{"label":"no"}]}]}"#,
        );
        assert!(matches!(action, ToolCallAction::Run), "confident ask verdict must allow: {action:?}");
    }

    #[test]
    fn allows_in_policy_call() {
        let hook = hook(Role::Translator);
        assert!(hook.deterministic("write", r#"{"path":"target/lib.rs","content":"fn f() {}"}"#).is_none());
        assert!(hook.deterministic("read", r#"{"path":"meta/plan.md"}"#).is_none());
    }

    #[test]
    fn non_json_args_pass_through() {
        let hook = hook(Role::Translator);
        // Malformed args must not panic the gateway; the tool itself will
        // surface the parse error.
        assert!(hook.deterministic("write", "not json").is_none());
    }

    #[test]
    fn allows_reads_of_source() {
        // Regression: the readonly rule once blocked reads of source/, so
        // the analyst could never see the codebase it must map.
        let hook = hook(Role::Analyst);
        assert!(hook.deterministic("read", r#"{"path":"source/source.py"}"#).is_none());
        assert!(hook.deterministic("read", r#"{"path":"source"}"#).is_none());
    }

    #[test]
    fn denies_tool_calls_past_budget() {
        let policy = GuardConfig {
            max_tool_calls: 2,
            ..GuardConfig::default()
        };
        let hook = GuardHook::new(Role::Validator, workspace(), policy, 32768);
        assert!(hook.budget_verdict().is_none(), "first call allowed");
        assert!(hook.budget_verdict().is_none(), "second call allowed");
        let verdict = hook.budget_verdict().expect("third call denied");
        assert!(verdict.contains("budget"));
        assert!(verdict.contains("verdict"));
    }

    #[test]
    fn zero_budget_disables_cap() {
        let hook = hook(Role::Validator);
        for _ in 0..5 {
            assert!(hook.budget_verdict().is_none());
        }
    }

    #[test]
    fn allows_small_read_below_threshold() {
        let dir = std::env::temp_dir().join(format!("guard-read-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("source")).unwrap();
        std::fs::write(dir.join("source/small.c"), "int x;\n".repeat(10)).unwrap();
        let hook = GuardHook::new(Role::Analyst, Workspace::new(dir), GuardConfig::default(), 32768);
        assert!(hook.deterministic("read", r#"{"path":"source/small.c"}"#).is_none());
    }

    #[test]
    fn skips_read_pushing_projection_over_threshold() {
        let dir = std::env::temp_dir().join(format!("guard-read-big-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("source")).unwrap();
        // At a 32768 window, the gate trips at 80%: ~26k tokens ~ 105k bytes.
        std::fs::write(dir.join("source/big.c"), "x".repeat(200_000)).unwrap();
        let hook = GuardHook::new(Role::Analyst, Workspace::new(dir), GuardConfig::default(), 32768);
        hook.projected_tokens.store(20_000, Ordering::Relaxed);
        let verdict = hook.deterministic("read", r#"{"path":"source/big.c"}"#).expect("big read denied");
        assert!(verdict.contains("context window"));
        assert!(verdict.contains("scope to one module"));
    }

    #[test]
    fn gate_fires_for_workspace_relative_path_from_any_cwd() {
        // Regression: the size check once resolved against the harness cwd,
        // so real-run paths (workspace-relative) never hit a file and the
        // gate silently never fired (c5 traces: 0 skips with 47 large
        // reads). The path here exists only under the workspace root, and
        // the test runs from whatever cwd the harness uses.
        let dir = std::env::temp_dir().join(format!("guard-read-rel-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("source")).unwrap();
        std::fs::write(dir.join("source/big.c"), "x".repeat(200_000)).unwrap();
        let hook = GuardHook::new(Role::Analyst, Workspace::new(dir), GuardConfig::default(), 32768);
        hook.projected_tokens.store(20_000, Ordering::Relaxed);
        let verdict = hook.deterministic("read", r#"{"path":"source/big.c"}"#).expect("gate must fire");
        assert!(verdict.contains("context window"));
    }

    #[test]
    fn non_read_tools_skip_the_size_gate() {
        let dir = std::env::temp_dir().join(format!("guard-read-bash-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let hook = GuardHook::new(Role::Translator, Workspace::new(dir.clone()), GuardConfig::default(), 32768);
        // A bash command touching a huge file is not a read; the gate is
        // read-only by design.
        let args = format!(r#"{{"command":"wc -l {}"}}"#, dir.display());
        assert!(hook.deterministic("bash", &args).is_none());
    }

    #[test]
    fn zero_num_ctx_disables_read_gate() {
        let dir = std::env::temp_dir().join(format!("guard-read-off-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("source")).unwrap();
        std::fs::write(dir.join("source/big.c"), "x".repeat(200_000)).unwrap();
        let hook = GuardHook::new(Role::Analyst, Workspace::new(dir), GuardConfig::default(), 0);
        hook.projected_tokens.store(20_000, Ordering::Relaxed);
        assert!(hook.deterministic("read", r#"{"path":"source/big.c"}"#).is_none());
    }
}
