//! Unit tests over the census fixtures plus a fake-child handshake test.
use crate::config::{PromptRequest, SessionConfig};
use crate::error::Error;
use crate::fixtures::*;
use crate::frame::{decode_line, Event, RawLine};
use crate::frame_payloads::{SessionEvent, StreamingBehavior};
use crate::session::Session;
use std::time::Duration;

#[test]
fn banner_line_is_skipped() {
    match decode_line(BANNER) {
        RawLine::Skipped(text) => assert_eq!(text, BANNER),
        RawLine::Event(_) => panic!("banner line decoded as a frame"),
    }
}

#[test]
fn empty_line_is_skipped() {
    assert!(matches!(decode_line(""), RawLine::Skipped(_)));
}

#[test]
fn ready_frame_census_line_decodes() {
    match decode_line(READY) {
        RawLine::Event(event) => match *event {
            Event::Ready(ready) => {
                assert_eq!(ready.protocol_version, 1);
                assert_eq!(ready.supported_protocol_versions, vec![1, 2]);
                assert_eq!(ready.max_frame_bytes, 1_048_576);
                assert_eq!(ready.max_reassembled_frame_bytes, 67_108_864);
            },
            other => panic!("wrong frame: {other:?}"),
        },
        RawLine::Skipped(text) => panic!("ready line skipped: {text}"),
    }
}

#[test]
fn response_frames_census_lines_decode_with_ids() {
    for line in [RESPONSE_NEGOTIATE, RESPONSE_SUBSCRIPTION, RESPONSE_PROMPT] {
        match decode_line(line) {
            RawLine::Event(event) => match *event {
                Event::Response(response) => {
                    assert!(response.success);
                    assert!(response.id.is_some());
                },
                other => panic!("wrong frame: {other:?}"),
            },
            RawLine::Skipped(text) => panic!("response line skipped: {text}"),
        }
    }
}

#[test]
fn prompt_result_census_line_decodes_camel_case() {
    match decode_line(PROMPT_RESULT) {
        RawLine::Event(event) => match *event {
            Event::PromptResult(result) => {
                assert_eq!(result.id.as_deref(), Some("3"));
                assert_eq!(result.status.as_deref(), Some("completed"));
                assert_eq!(result.agent_invoked, Some(true));
                assert_eq!(result.session_settled, Some(true));
            },
            other => panic!("wrong frame: {other:?}"),
        },
        RawLine::Skipped(text) => panic!("prompt_result line skipped: {text}"),
    }
}

#[test]
fn session_event_census_lines_decode_as_typed_or_other() {
    for line in [AGENT_START, MESSAGE_START, MESSAGE_END_ASSISTANT, TURN_END, AGENT_END, ADVISOR_COST_CHANGED] {
        match decode_line(line) {
            RawLine::Event(event) => match *event {
                Event::Other(_) => {},
                other => panic!("unexpected typed frame: {other:?}"),
            },
            RawLine::Skipped(text) => panic!("session line skipped: {text}"),
        }
    }
}

#[test]
fn extension_ui_request_census_line_decodes() {
    match decode_line(EXTENSION_UI_REQUEST) {
        RawLine::Event(event) => match *event {
            Event::ExtensionUiRequest(request) => {
                assert_eq!(request.method.as_deref(), Some("setWidget"));
                assert_eq!(request.extra.get("widgetKey").and_then(|value| value.as_str()), Some("autoresearch"));
            },
            other => panic!("wrong frame: {other:?}"),
        },
        RawLine::Skipped(text) => panic!("extension_ui_request line skipped: {text}"),
    }
}

#[test]
fn subagent_frames_route_by_agent_and_id() {
    match decode_line(SUBAGENT_LIFECYCLE) {
        RawLine::Event(event) => match *event {
            Event::SubagentLifecycle(frame) => {
                assert_eq!(frame.payload.agent, "recodeagent-analyzer");
                assert_eq!(frame.payload.id, "FranticSole");
            },
            other => panic!("wrong frame: {other:?}"),
        },
        RawLine::Skipped(text) => panic!("lifecycle line skipped: {text}"),
    }
    match decode_line(SUBAGENT_PROGRESS) {
        RawLine::Event(event) => match *event {
            Event::SubagentProgress(frame) => {
                // Route by payload.agent. Progress carries it at top level.
                assert_eq!(frame.payload.agent, "recodeagent-analyzer");
                assert_eq!(frame.payload.progress.last_intent.as_deref(), Some("reading src"));
                assert_eq!(frame.payload.progress.recent_output, vec!["line one", "line two"]);
            },
            other => panic!("wrong frame: {other:?}"),
        },
        RawLine::Skipped(text) => panic!("progress line skipped: {text}"),
    }
    match decode_line(SUBAGENT_EVENT) {
        RawLine::Event(event) => match *event {
            Event::SubagentEvent(frame) => {
                // The payload has NO agent field. Route by payload.id.
                assert_eq!(frame.payload.id, "FranticSole");
                assert_eq!(
                    frame.payload.event.extra.get("type").and_then(|value| value.as_str()),
                    Some("message_update")
                );
            },
            other => panic!("wrong frame: {other:?}"),
        },
        RawLine::Skipped(text) => panic!("subagent_event line skipped: {text}"),
    }
}

#[test]
fn unknown_frame_type_survives_as_other() {
    let line = r#"{"type": "brand_new_frame_from_a_future_omp", "magic": 42}"#;
    match decode_line(line) {
        RawLine::Event(event) => match *event {
            Event::Other(other) => {
                assert_eq!(other.extra.get("magic").and_then(|value| value.as_i64()), Some(42));
            },
            other => panic!("wrong frame: {other:?}"),
        },
        RawLine::Skipped(text) => panic!("unknown frame dropped: {text}"),
    }
}

#[test]
fn prompt_request_serializes_streaming_behavior() {
    let mut request = PromptRequest::new("hello");
    assert_eq!(request.to_value(), serde_json::json!({"type": "prompt", "message": "hello"}));
    request.streaming_behavior = Some(StreamingBehavior::Steer);
    assert_eq!(
        request.to_value(),
        serde_json::json!({"type": "prompt", "message": "hello", "streamingBehavior": "steer"})
    );
}

#[test]
fn session_config_argv_carries_model_and_passthrough() {
    let config = SessionConfig {
        model: Some("ashaiballant/qwen3.8-27b-df2".to_string()),
        extra_args: vec!["--no-session".to_string()],
        ..SessionConfig::default()
    };
    assert_eq!(
        config.argv(),
        vec![
            "--mode".to_string(),
            "rpc".to_string(),
            "--model=ashaiballant/qwen3.8-27b-df2".to_string(),
            "--approval-mode=yolo".to_string(),
            "--no-session".to_string(),
        ]
    );
}

// --- Handshake and correlation against a fake child -----------------------

/// Script a fake child that replays the census handshake lines. The census
/// ids are 1, 2, 3 (negotiate, subscription, prompt). The Session assigns
/// 1 to negotiate and 2 to prompt, so the replay shifts the prompt ids.
fn fake_child_script() -> String {
    let response_prompt = r#"{"command": "prompt", "id": "2", "success": true, "type": "response"}"#;
    let prompt_result =
        r#"{"agentInvoked": true, "id": "2", "sessionSettled": true, "status": "completed", "type": "prompt_result"}"#;
    [READY, RESPONSE_NEGOTIATE, RESPONSE_SUBSCRIPTION, response_prompt, prompt_result, BANNER].join("\n")
}

#[tokio::test]
async fn handshake_and_request_id_correlation_over_fake_child() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let script_path = temp_dir.path().join("fake-omp.sh");
    std::fs::write(&script_path, format!("#!/bin/sh\ncat <<'EOF'\n{}\nEOF\nsleep 30\n", fake_child_script()))
        .expect("write script");
    std::fs::set_permissions(&script_path, std::os::unix::fs::PermissionsExt::from_mode(0o755)).expect("chmod");

    let config = SessionConfig {
        program: script_path.to_string_lossy().to_string(),
        working_dir: temp_dir.path().to_path_buf(),
        ..SessionConfig::default()
    };
    let mut session = Session::spawn(&config).expect("spawn fake child");

    // ready arrives first, before any request.
    let ready = session.next_event_within(Duration::from_secs(5)).await;
    assert!(matches!(ready, Some(Event::Ready(_))), "expected ready frame");

    // negotiate: the echo id correlates by string.
    let id =
        session.request(serde_json::json!({"type": "negotiate_protocol", "protocolVersion": 2})).await.expect("send");
    assert_eq!(id, "1");
    let response = wait_for_response(&mut session, &id).await;
    assert!(response.success);
    assert_eq!(response.command.as_deref(), Some("negotiate_protocol"));

    // prompt: response ack then prompt_result with the SAME id.
    let prompt_id = session.prompt(PromptRequest::new("Reply with exactly: OK")).await.expect("send prompt");
    assert_eq!(prompt_id, "2");
    let response = wait_for_response(&mut session, &prompt_id).await;
    assert!(response.success);
    let result = wait_for_prompt_result(&mut session, &prompt_id).await;
    assert_eq!(result.id.as_deref(), Some(prompt_id.as_str()));
    assert_eq!(result.status.as_deref(), Some("completed"));

    // Dropping the Session kills the child (kill_on_drop). The reader task
    // ends on the closed pipe.
}

async fn wait_for_response(session: &mut Session, request_id: &str) -> crate::frame_payloads::ResponseFrame {
    loop {
        match session.next_event_within(Duration::from_secs(5)).await {
            Some(Event::Response(response)) if response.id.as_deref() == Some(request_id) => return response,
            Some(_) => continue,
            None => panic!("event stream ended while waiting for response {request_id}"),
        }
    }
}

async fn wait_for_prompt_result(session: &mut Session, request_id: &str) -> crate::frame_payloads::PromptResultFrame {
    loop {
        match session.next_event_within(Duration::from_secs(5)).await {
            Some(Event::PromptResult(result)) if result.id.as_deref() == Some(request_id) => return result,
            Some(_) => continue,
            None => panic!("event stream ended while waiting for prompt_result {request_id}"),
        }
    }
}

#[test]
fn error_variants_display_for_callers() {
    let error = Error::FrameTooLarge(2048);
    assert!(error.to_string().contains("2048"));
    let error = Error::RequestRejected {
        request_id: "7".to_string(),
        detail: "bad level".to_string(),
    };
    assert!(error.to_string().contains("bad level"));
}

#[test]
fn spawn_failure_keeps_io_error_source() {
    let config = SessionConfig {
        program: "/no/such/omp-binary".to_string(),
        ..SessionConfig::default()
    };
    let error = Session::spawn(&config).err().expect("spawn fails");
    match error {
        Error::HandshakeFailed(Some(io_error)) => {
            assert_eq!(io_error.kind(), std::io::ErrorKind::NotFound);
            assert!(io_error.to_string().contains("No such file"));
        },
        other => panic!("wrong error: {other:?}"),
    }
}

#[test]
fn session_event_flattens_unknown_fields() {
    let event = SessionEvent::default();
    assert!(event.extra.is_empty());
}
