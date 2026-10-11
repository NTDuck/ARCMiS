//! `frames` is the wire vocabulary over the omp RPC child (ADR 0035).
//!
//! The xoannova `omp-rpc` SDK decodes the wire. This module keeps the
//! local frame types the conductor and frame sink consume, plus the
//! mapping from one SDK event onto them.
//!
//! Field names follow the live wire: ALL camelCase. A missing
//! `#[serde(rename)]` does not fail parsing when a catch-all variant
//! absorbs the frame — it silently drops the data. The shapes come
//! from a live census against omp 18.6.1, never from memory.

use serde::Deserialize;
use serde_json::Value;

use omp_rpc::Event as SdkEvent;
use omp_rpc::RpcNotification;

/// One decoded server frame. Unknown shapes stay typed as [`Event::Other`].
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Ready(ReadyFrame),
    Response(ResponseFrame),
    PromptResult(PromptResultFrame),
    SubagentLifecycle(SubagentLifecycleFrame),
    SubagentProgress(SubagentProgressFrame),
    SubagentEvent(SubagentEventFrame),
    ExtensionUiRequest(ExtensionUiRequestFrame),
    #[serde(untagged)]
    Other(SessionEvent),
}

/// Map one SDK event onto the local vocabulary.
pub fn map_sdk_event(event: SdkEvent) -> Event {
    match event {
        SdkEvent::Notification(notification) => map_notification(notification),
        SdkEvent::UnmatchedResponse(response) => Event::Response(ResponseFrame {
            id: response.id,
            command: Some(response.command),
            success: response.success,
            data: response.data,
            error: response.error.map(Value::String),
        }),
        SdkEvent::Undecodable {
            raw,
            error,
        } => {
            // WHY a catch-all: omp adds frame types between releases and
            // the SDK refuses the unknown shape. Dropping them silently
            // hangs wait loops (verified in a prior integration), so
            // unknown frames stay reachable as the raw payload and log
            // at debug.
            tracing::debug!(error = %error, "kept undecodable frame");
            Event::Other(raw_event(raw))
        },
    }
}

fn map_notification(notification: RpcNotification) -> Event {
    match notification {
        RpcNotification::Ready(ready) => Event::Ready(ReadyFrame {
            protocol_version: ready.protocol_version.unwrap_or(1) as u32,
            supported_protocol_versions: ready
                .supported_protocol_versions
                .map(|versions| versions.iter().map(|version| *version as u32).collect())
                .unwrap_or_default(),
            max_frame_bytes: ready.max_frame_bytes.unwrap_or(0) as u64,
            max_reassembled_frame_bytes: ready.max_reassembled_frame_bytes.unwrap_or(0) as u64,
        }),
        RpcNotification::PromptResult(result) => Event::PromptResult(PromptResultFrame {
            id: result.id,
            agent_invoked: Some(result.agent_invoked),
            status: Some(result.status.as_str().to_owned()),
            session_settled: Some(result.session_settled),
        }),
        RpcNotification::SubagentLifecycle(frame) => Event::SubagentLifecycle(SubagentLifecycleFrame {
            payload: SubagentLifecyclePayload {
                id: frame.payload.id,
                agent: frame.payload.agent,
                agent_source: Some(frame.payload.agent_source.as_str().to_owned()),
                description: frame.payload.description,
                status: frame.payload.status.as_str().to_owned(),
                session_file: frame.payload.session_file,
                parent_tool_call_id: frame.payload.parent_tool_call_id,
                index: frame.payload.index as u32,
                detached: frame.payload.detached,
            },
        }),
        RpcNotification::SubagentProgress(frame) => Event::SubagentProgress(SubagentProgressFrame {
            payload: SubagentProgressPayload {
                index: frame.payload.index as u32,
                agent: frame.payload.agent,
                agent_source: Some(frame.payload.agent_source.as_str().to_owned()),
                task: Some(frame.payload.task),
                assignment: frame.payload.assignment,
                progress: serde_json::from_value(Value::Object(frame.payload.progress.clone())).unwrap_or_default(),
                session_file: frame.payload.session_file,
                detached: frame.payload.detached,
            },
        }),
        RpcNotification::SubagentEvent(frame) => Event::SubagentEvent(SubagentEventFrame {
            payload: SubagentEventPayload {
                id: frame.payload.id,
                event: raw_event(serde_json::to_value(&frame.payload.event).unwrap_or(Value::Null)),
            },
        }),
        RpcNotification::ExtensionUiRequest(request) => Event::ExtensionUiRequest(
            serde_json::from_value(serde_json::to_value(&request).unwrap_or(Value::Null)).unwrap_or_default(),
        ),
        RpcNotification::RpcAgentEvent(agent_event) => {
            Event::Other(raw_event(serde_json::to_value(&agent_event).unwrap_or(Value::Null)))
        },
        RpcNotification::Unknown(raw) => Event::Other(raw_event(raw)),
        other => Event::Other(raw_event(serde_json::to_value(&other).unwrap_or(Value::Null))),
    }
}

/// Fold one raw frame value into the catch-all payload.
fn raw_event(value: Value) -> SessionEvent {
    let extra = match value {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    SessionEvent {
        extra,
    }
}

/// Catch-all payload for frame types this crate does not model.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SessionEvent {
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// First frame on stdout. Carries the protocol version and frame limits.
#[derive(Debug, Clone, Deserialize)]
pub struct ReadyFrame {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
    #[serde(rename = "supportedProtocolVersions")]
    pub supported_protocol_versions: Vec<u32>,
    #[serde(rename = "maxFrameBytes")]
    pub max_frame_bytes: u64,
    #[serde(rename = "maxReassembledFrameBytes")]
    pub max_reassembled_frame_bytes: u64,
}

/// Command response. `id` echoes the request id the host sent.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseFrame {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
}

/// Terminal state of one prompt command.
#[derive(Debug, Clone, Deserialize)]
pub struct PromptResultFrame {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "agentInvoked", default)]
    pub agent_invoked: Option<bool>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(rename = "sessionSettled", default)]
    pub session_settled: Option<bool>,
}

/// Registry transition for one subagent. Route by `payload.agent`.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentLifecycleFrame {
    pub payload: SubagentLifecyclePayload,
}

/// Body of `subagent_lifecycle`.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentLifecyclePayload {
    pub id: String,
    pub agent: String,
    #[serde(rename = "agentSource", default)]
    pub agent_source: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub status: String,
    #[serde(rename = "sessionFile", default)]
    pub session_file: Option<String>,
    #[serde(rename = "parentToolCallId", default)]
    pub parent_tool_call_id: Option<String>,
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub detached: Option<bool>,
}

/// Full progress snapshot for one subagent. Route by `payload.agent`.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentProgressFrame {
    #[serde(flatten)]
    pub payload: SubagentProgressPayload,
}

/// Body of `subagent_progress`. The `progress` field mirrors the Agent Hub row.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentProgressPayload {
    #[serde(default)]
    pub index: u32,
    pub agent: String,
    #[serde(rename = "agentSource", default)]
    pub agent_source: Option<String>,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub assignment: Option<String>,
    pub progress: AgentProgress,
    #[serde(rename = "sessionFile", default)]
    pub session_file: Option<String>,
    #[serde(default)]
    pub detached: Option<bool>,
}

/// Live counters and current activity of one agent.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgentProgress {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(rename = "lastIntent", default)]
    pub last_intent: Option<String>,
    #[serde(rename = "currentTool", default)]
    pub current_tool: Option<String>,
    #[serde(rename = "currentToolArgs", default)]
    pub current_tool_args: Option<String>,
    #[serde(rename = "recentOutput", default)]
    pub recent_output: Vec<String>,
    #[serde(rename = "toolCount", default)]
    pub tool_count: u32,
    #[serde(default)]
    pub tokens: u64,
    #[serde(rename = "contextTokens", default)]
    pub context_tokens: Option<u64>,
    #[serde(rename = "contextWindow", default)]
    pub context_window: Option<u64>,
    #[serde(default)]
    pub cost: f64,
    #[serde(rename = "durationMs", default)]
    pub duration_ms: u64,
}

/// One forwarded session event of one subagent. The payload has NO `agent`
/// field. Route by `payload.id` through an id-to-agent map.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentEventFrame {
    pub payload: SubagentEventPayload,
}

/// Body of `subagent_event`. The `event` field is a full session frame.
#[derive(Debug, Clone, Deserialize)]
pub struct SubagentEventPayload {
    pub id: String,
    pub event: SessionEvent,
}

/// Dialog request from an extension. Answer with `extension_ui_response`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExtensionUiRequestFrame {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use omp_rpc::AgentSource;
    use omp_rpc::PromptResultEvent;
    use omp_rpc::PromptStatus;
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
        assert_eq!(mapped.extra.get("type").and_then(Value::as_str), Some("brand_new_frame"));
        assert_eq!(mapped.extra.get("note").and_then(Value::as_str), Some("kept"));
    }
}
