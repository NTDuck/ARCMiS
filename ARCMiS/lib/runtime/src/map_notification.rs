//! Map one SDK notification onto the local frame vocabulary.

use omp_rpc::RpcNotification;
use serde_json::Value;

use crate::frames::Event;
use crate::prompt_error_detail::PromptErrorDetail;
use crate::prompt_result::PromptResultFrame;
use crate::ready::ReadyFrame;
use crate::session_event::SessionEvent;
use crate::subagent_event::{SubagentEventFrame, SubagentEventPayload};
use crate::subagent_lifecycle::{SubagentLifecycleFrame, SubagentLifecyclePayload};
use crate::subagent_progress::{SubagentProgressFrame, SubagentProgressPayload};

/// Map one SDK notification onto the local frame vocabulary.
pub fn map_notification(notification: RpcNotification) -> Event {
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
            error: result.error.map(|error| PromptErrorDetail {
                message: error.message,
                retryable: error.retryable,
                provider: error.provider,
                model: error.model,
                http_status: error.http_status,
            }),
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
