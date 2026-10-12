//! Integration tests for the gateway crate.

use mas_gateway::Gateway;
use mas_gateway::ToolCall;
use mas_gateway::ToolVerdict;

fn call(tool: &str, args: &str) -> ToolCall {
    ToolCall {
        tool: tool.to_owned(),
        args: args.to_owned(),
    }
}

#[test]
fn permissive_gateway_allows_everything() {
    let gateway = Gateway::permissive();
    assert_eq!(gateway.judge(&call("bash", "rm -rf /")), ToolVerdict::Allow);
}

#[test]
fn deny_pattern_blocks_matching_call() {
    let gateway = Gateway::new(vec!["bash:rm -rf".to_owned()], Vec::new());
    let verdict = gateway.judge(&call("bash", "rm -rf /tmp"));
    assert!(matches!(verdict, ToolVerdict::Deny { .. }));
}

#[test]
fn deny_dominates_allow() {
    let gateway = Gateway::new(vec!["bash:rm".to_owned()], vec!["bash:ls".to_owned()]);
    let verdict = gateway.judge(&call("bash", "rm -rf /"));
    assert!(matches!(verdict, ToolVerdict::Deny { .. }));
}

#[test]
fn allow_pattern_permits_matching_call() {
    let gateway = Gateway::new(Vec::new(), vec!["bash:ls".to_owned()]);
    assert_eq!(gateway.judge(&call("bash", "ls -la")), ToolVerdict::Allow);
}

#[test]
fn empty_allow_list_blocks_unmatched_calls() {
    let gateway = Gateway::new(Vec::new(), vec!["bash:ls".to_owned()]);
    let verdict = gateway.judge(&call("bash", "rm -rf /"));
    assert!(matches!(verdict, ToolVerdict::Deny { .. }));
}

#[test]
fn tool_name_must_match() {
    let gateway = Gateway::new(vec!["bash:rm".to_owned()], Vec::new());
    assert_eq!(gateway.judge(&call("edit", "rm -rf /")), ToolVerdict::Allow);
}
