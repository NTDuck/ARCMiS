//! Router: classify one task description into a specialist role via a
//! keyword pass first (free, deterministic) and the router model second.
//! The keyword pass exists so the common cases never cost a model call.

use agents::Role;

/// Classify a task description. Returns the role the router picked.
///
/// The deterministic pass covers the verbs each role's prompt owns. When
/// nothing matches, the caller falls back to the router model.
#[must_use]
pub fn route_by_keywords(description: &str) -> Option<Role> {
    let text = description.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|word| text.contains(word));

    if has(&["diagnose", "diagnosis", "classify the failure", "root cause"]) && !has(&["fix", "repair"]) {
        return Some(Role::FailureAnalyst);
    }
    if has(&["repair", "fix the", "fix one", "apply the suggested action"]) {
        return Some(Role::Repairer);
    }
    if has(&["validate", "check the batch", "review the batch", "verify the batch"]) {
        return Some(Role::Validator);
    }
    if has(&["critique", "adversarial review", "final review"]) {
        return Some(Role::Critic);
    }
    if has(&["source map", "survey the codebase", "list the modules", "read the codebase"]) {
        return Some(Role::Analyst);
    }
    if has(&["brief", "migration contract", "gap decisions", "target layout"]) {
        return Some(Role::Architect);
    }
    if has(&["batch", "batches", "plan.json", "dependency batches"]) {
        return Some(Role::Planner);
    }
    if has(&["translate", "migrate module", "write the target module", "port module"]) {
        return Some(Role::Translator);
    }
    if has(&["test", "characterization", "run the test command"]) {
        return Some(Role::Tester);
    }
    if has(&["fleet", "model promotion", "promote", "demote"]) {
        return Some(Role::FleetAnalyst);
    }
    None
}
