//! Integration tests for the frames crate.

use mas_frames::map_sdk_event;
use mas_frames::Event;
use omp_rpc::AgentSource;
use omp_rpc::Event as SdkEvent;
use omp_rpc::PromptError;
use omp_rpc::PromptResultEvent;
use omp_rpc::PromptStatus;
use omp_rpc::RpcNotification;
use omp_rpc::RpcResponse;
use omp_rpc::SubagentLifecycleEvent;
use omp_rpc::SubagentLifecyclePayload as SdkLifecyclePayload;
use omp_rpc::SubagentLifecycleStatus;

fn lifecycle_frame() -> SdkEvent {
    SdkEvent::Notification(RpcNotification::SubagentLifecycle(SubagentLifecycleEvent {
        payload: SdkLifecyclePayload {
            id: "FranticSole".to_string(),
            agent: "recodeagent-analyzer".to_string(),
            agent_source: AgentSource::Project,
            status: SubagentLifecycleStatus::Started,
            index: 3,
            description: None,
            session_file: None,
            parent_tool_call_id: Some("call_9".to_string()),
            detached: None,
        },
    }))
}

#[test]
fn subagent_lifecycle_keeps_the_routing_fields() {
    let frame = map_sdk_event(lifecycle_frame());
    let Event::SubagentLifecycle(mapped) = frame else {
        panic!("expected a subagent lifecycle frame");
    };
    assert_eq!(mapped.payload.agent, "recodeagent-analyzer");
    assert_eq!(mapped.payload.agent_source.as_deref(), Some("project"));
    assert_eq!(mapped.payload.status, "started");
    assert_eq!(mapped.payload.index, 3);
    assert_eq!(mapped.payload.parent_tool_call_id.as_deref(), Some("call_9"));
}

#[test]
fn prompt_result_maps_the_status_and_id() {
    let frame = SdkEvent::Notification(RpcNotification::PromptResult(PromptResultEvent {
        agent_invoked: true,
        status: PromptStatus::Error,
        session_settled: true,
        id: Some("req_2".to_string()),
        error: None,
    }));
    let Event::PromptResult(mapped) = map_sdk_event(frame) else {
        panic!("expected a prompt result frame");
    };
    assert_eq!(mapped.status.as_deref(), Some("error"));
    assert_eq!(mapped.id.as_deref(), Some("req_2"));
    assert_eq!(mapped.session_settled, Some(true));
    assert_eq!(mapped.error, None);
}

#[test]
fn prompt_result_keeps_the_error_detail() {
    let frame = SdkEvent::Notification(RpcNotification::PromptResult(PromptResultEvent {
        agent_invoked: true,
        status: PromptStatus::Error,
        session_settled: true,
        id: Some("req_3".to_string()),
        error: Some(omp_rpc::PromptError {
            message: "Agent is already processing".to_string(),
            retryable: true,
            provider: Some("local-engine".to_string()),
            model: Some("qwen3.8-27b".to_string()),
            http_status: None,
        }),
    }));
    let Event::PromptResult(mapped) = map_sdk_event(frame) else {
        panic!("expected a prompt result frame");
    };
    let detail = mapped.error.expect("expected the error detail");
    assert_eq!(detail.message, "Agent is already processing");
    assert!(detail.retryable);
    assert_eq!(detail.provider.as_deref(), Some("local-engine"));
    assert_eq!(detail.model.as_deref(), Some("qwen3.8-27b"));
    assert_eq!(detail.http_status, None);
}

#[test]
fn unmatched_response_becomes_a_response_frame() {
    let frame = SdkEvent::UnmatchedResponse(RpcResponse {
        command: "prompt".to_string(),
        success: false,
        id: Some("req_4".to_string()),
        data: None,
        error: Some("model exploded".to_string()),
        code: Some("prompt_failed".to_string()),
    });
    let Event::Response(mapped) = map_sdk_event(frame) else {
        panic!("expected a response frame");
    };
    assert_eq!(mapped.command.as_deref(), Some("prompt"));
    assert!(!mapped.success);
    assert_eq!(mapped.id.as_deref(), Some("req_4"));
}

#[test]
fn undecodable_frames_stay_reachable() {
    let frame = SdkEvent::Undecodable {
        raw: serde_json::json!({ "type": "brand_new_frame", "note": "kept" }),
        error: "no decoder".to_string(),
    };
    let Event::Other(mapped) = map_sdk_event(frame) else {
        panic!("expected a catch-all frame");
    };
    assert_eq!(mapped.extra.get("type").and_then(serde_json::Value::as_str), Some("brand_new_frame"));
    assert_eq!(mapped.extra.get("note").and_then(serde_json::Value::as_str), Some("kept"));
}
