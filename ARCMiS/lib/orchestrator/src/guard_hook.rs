//! Jev-as-a-guard: the tool gateway as an `on_tool_call` hook. Every call
//! from every agent passes the deterministic tier first — role allowlist,
//! path policy (`source/` read-only), deny patterns for bash. Rejections
//! return `ToolCallAction::Skip(feedback)` so the model sees the reason and
//! can self-correct; the breaker counts repeats. Calls the deterministic
//! tier marks `Ask` fall to the model-arbitration tier (the Jev slot),
//! which is deny-by-default until an arbiter is configured
//! (`guard.ask_model`).

use crate::guard::Guard;
use agents::util::config::GuardConfig;
use agents::Role;
use rig::agent::hook::{AgentHook, HookContext, ToolCall, ToolCallAction};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;

/// The tool gateway over one role. Built per delegation; the registry does
/// not attach it because the role is only known at dispatch time.
#[derive(Clone)]
pub struct GuardHook {
    inner: Guard,
    policy: GuardConfig,
    /// Consecutive rejections inside this delegation; the manager sees the
    /// count when the delegation ends.
    rejections: Arc<AtomicUsize>,
    /// Last rejection feedback (for the delegation report).
    last_rejection: Arc<Mutex<Option<String>>>,
    /// Tool calls executed so far in this delegation; the policy cap denies
    /// further calls once this crosses `max_tool_calls`.
    calls: Arc<AtomicUsize>,
}

impl GuardHook {
    /// Build the gateway for one role under the run policy.
    #[must_use]
    pub fn new(role: Role, workspace: blackboard::Workspace, policy: GuardConfig) -> Self {
        Self {
            inner: Guard::new(role.allowed_tools().to_vec(), workspace),
            policy,
            rejections: Arc::new(AtomicUsize::new(0)),
            last_rejection: Arc::new(Mutex::new(None)),
            calls: Arc::new(AtomicUsize::new(0)),
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
            if let Err(error) = self.inner.permits_path(tool_name, &PathBuf::from(&path)) {
                return Some(error.to_string());
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
    async fn on_tool_call(&self, _ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        let verdict = self.deterministic(event.tool_name, event.args).or_else(|| self.budget_verdict());
        if let Some(reason) = verdict {
            self.rejections.fetch_add(1, Ordering::Relaxed);
            if let Ok(mut slot) = self.last_rejection.lock() {
                *slot = Some(format!("{}: {}", event.tool_name, reason));
            }
            // The Jev slot: Ask-class calls would route to the arbiter model
            // here when guard.ask_model is on. With one model there is no
            // independent arbiter, so Ask falls through to deny.
            return ToolCallAction::Skip(format!(
                "GUARD REFUSAL: {reason}. Stay inside the role's tools and the workspace layout."
            ));
        }
        ToolCallAction::Run
    }
}

impl GuardHook {
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

#[cfg(test)]
mod tests {
    use super::*;
    use blackboard::Workspace;

    fn workspace() -> Workspace {
        let dir = std::env::temp_dir().join(format!("guard-hook-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(dir.join("target"));
        let _ = std::fs::create_dir_all(dir.join("source"));
        Workspace::new(dir)
    }

    #[test]
    fn denies_tool_outside_role_allowlist() {
        let hook = GuardHook::new(Role::Validator, workspace(), GuardConfig::default());
        let verdict = hook.deterministic("write", r#"{"path":"target/x.rs","content":"fn main() {}"}"#);
        assert!(verdict.is_some());
        assert!(verdict.unwrap().contains("allowlist"));
    }

    #[test]
    fn denies_write_into_source() {
        let hook = GuardHook::new(Role::Translator, workspace(), GuardConfig::default());
        let verdict = hook.deterministic("write", r#"{"path":"source/main.c","content":"int x;"}"#);
        assert!(verdict.is_some());
    }

    #[test]
    fn denies_bash_deny_pattern() {
        let hook = GuardHook::new(Role::Translator, workspace(), GuardConfig::default());
        let verdict = hook.deterministic("bash", r#"{"command":"rm -rf /"}"#);
        assert!(verdict.is_some());
        assert!(verdict.unwrap().contains("denied by policy"));
    }

    #[test]
    fn allows_in_policy_call() {
        let hook = GuardHook::new(Role::Translator, workspace(), GuardConfig::default());
        assert!(hook.deterministic("write", r#"{"path":"target/lib.rs","content":"fn f() {}"}"#).is_none());
        assert!(hook.deterministic("read", r#"{"path":"meta/plan.md"}"#).is_none());
    }

    #[test]
    fn non_json_args_pass_through() {
        let hook = GuardHook::new(Role::Translator, workspace(), GuardConfig::default());
        // Malformed args must not panic the gateway; the tool itself will
        // surface the parse error.
        assert!(hook.deterministic("write", "not json").is_none());
    }

    #[test]
    fn denies_tool_calls_past_budget() {
        let policy = GuardConfig {
            max_tool_calls: 2,
            ..GuardConfig::default()
        };
        let hook = GuardHook::new(Role::Validator, workspace(), policy);
        assert!(hook.budget_verdict().is_none(), "first call allowed");
        assert!(hook.budget_verdict().is_none(), "second call allowed");
        let verdict = hook.budget_verdict().expect("third call denied");
        assert!(verdict.contains("budget"));
        assert!(verdict.contains("verdict"));
    }

    #[test]
    fn zero_budget_disables_cap() {
        let hook = GuardHook::new(Role::Validator, workspace(), GuardConfig::default());
        for _ in 0..5 {
            assert!(hook.budget_verdict().is_none());
        }
    }
}
