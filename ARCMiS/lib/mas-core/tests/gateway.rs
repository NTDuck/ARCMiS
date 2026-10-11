//! Integration tests for the gateway policy: deny patterns, allow
//! dominance, and deny-wins-when-both-match.

use mas_core::gateway::Gateway;
use mas_core::gateway::ToolCall;
use mas_core::gateway::ToolVerdict;

fn call(tool: &str, args: &str) -> ToolCall {
    ToolCall {
        tool: tool.to_owned(),
        args: args.to_owned(),
    }
}

#[test]
fn permissive_gateway_allows_everything() {
    let gateway = Gateway::permissive();
    assert_eq!(gateway.judge(&call("bash", "ls")), ToolVerdict::Allow);
}

#[test]
fn deny_pattern_uses_tool_substring_form() {
    let gateway = Gateway::new(vec!["bash:rm -rf".to_owned()], Vec::new());
    assert_eq!(
        gateway.judge(&call("bash", "rm -rf /")),
        ToolVerdict::Deny {
            matched: "bash:rm -rf".to_owned()
        }
    );
    assert_eq!(gateway.judge(&call("bash", "ls")), ToolVerdict::Allow);
}

#[test]
fn deny_matches_other_tools_not_at_all() {
    let gateway = Gateway::new(vec!["bash:rm -rf".to_owned()], Vec::new());
    assert_eq!(gateway.judge(&call("edit", "rm -rf")), ToolVerdict::Allow);
}

#[test]
fn allowlist_blocks_uncatalogued_calls() {
    let gateway = Gateway::new(Vec::new(), vec!["bash:git ".to_owned()]);
    assert_eq!(gateway.judge(&call("bash", "git status")), ToolVerdict::Allow);
    assert!(matches!(gateway.judge(&call("bash", "curl example.com")), ToolVerdict::Deny { .. }));
}

#[test]
fn deny_beats_allow_when_both_match() {
    let gateway = Gateway::new(vec!["bash:rm".to_owned()], vec!["bash:rm -rf cache".to_owned()]);
    assert!(matches!(gateway.judge(&call("bash", "rm -rf cache")), ToolVerdict::Deny { .. }));
}
