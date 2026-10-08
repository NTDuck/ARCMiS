//! `event` folds RPC frames into UI state lines.

use crate::store::Store;
use omprpc::frame::ServerFrame;

/// Apply one frame. Returns true when the UI should redraw.
pub fn apply_frame(store: &mut Store, frame: &ServerFrame) -> bool {
    match frame {
        ServerFrame::Response(response) => {
            if !response.success {
                let error = response
                    .error
                    .as_ref()
                    .map(serde_json::Value::to_string)
                    .unwrap_or_else(|| "unknown error".to_string());
                store.orchestrator.push_line(format!("[error] {error}"));
                return true;
            }
            false
        },
        ServerFrame::AgentStart(_) => {
            store.busy = true;
            store.orchestrator.push_line("[run] model turn started".to_string());
            true
        },
        ServerFrame::AgentEnd(_) => {
            store.busy = false;
            store.orchestrator.push_line("[run] model turn ended".to_string());
            true
        },
        ServerFrame::MessageStart(event) => apply_message_start(store, &event.extra),
        ServerFrame::MessageUpdate(event) => apply_message_update(store, &event.extra),
        ServerFrame::MessageEnd(event) => apply_message_end(store, &event.extra),
        ServerFrame::ToolExecutionStart(event) => apply_tool_start(store, &event.extra),
        ServerFrame::ToolExecutionEnd(event) => apply_tool_end(store, &event.extra),
        ServerFrame::SubagentLifecycle(frame) => apply_subagent_lifecycle(store, frame),
        ServerFrame::SubagentProgress(frame) => apply_subagent_progress(store, frame),
        ServerFrame::SubagentEvent(frame) => {
            let id = frame.payload.id.clone();
            apply_subagent_event(store, &id, &frame.payload.event.extra)
        },
        ServerFrame::IrcMessage(event) => apply_irc_message(store, &event.extra),
        ServerFrame::Notice(event) => {
            let message = event.extra.get("message").and_then(serde_json::Value::as_str).unwrap_or("notice");
            store.notice = message.to_string();
            store.orchestrator.push_line(format!("[notice] {message}"));
            true
        },
        ServerFrame::PromptResult(prompt_result) => {
            let status = prompt_result.status.as_deref().unwrap_or("unknown");
            store.busy = false;
            store.orchestrator.push_line(format!("[run] prompt {status}"));
            true
        },
        ServerFrame::SessionSettled(_) => {
            store.busy = false;
            true
        },
        _ => false,
    }
}

fn text_of(event: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    let message = event.get("message")?;
    let content = message.get("content")?;
    let parts = content.as_array()?;
    let mut text = String::new();
    for part in parts {
        if part.get("type").and_then(serde_json::Value::as_str) == Some("text") {
            if let Some(piece) = part.get("text").and_then(serde_json::Value::as_str) {
                text.push_str(piece);
            }
        }
    }
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn apply_message_start(store: &mut Store, event: &serde_json::Map<String, serde_json::Value>) -> bool {
    if let Some(text) = text_of(event) {
        for line in text.lines() {
            store.orchestrator.push_line(line.to_string());
        }
        return true;
    }
    false
}

fn apply_message_update(store: &mut Store, event: &serde_json::Map<String, serde_json::Value>) -> bool {
    let Some(assistant_event) = event.get("assistantMessageEvent") else {
        return false;
    };
    let kind = assistant_event.get("type").and_then(serde_json::Value::as_str);
    if kind == Some("text_delta") {
        if let Some(delta) = assistant_event.get("delta").and_then(serde_json::Value::as_str) {
            for line in delta.lines() {
                if !line.trim().is_empty() {
                    store.orchestrator.push_line(line.to_string());
                }
            }
            return true;
        }
    }
    false
}

fn apply_message_end(store: &mut Store, event: &serde_json::Map<String, serde_json::Value>) -> bool {
    if let Some(text) = text_of(event) {
        for line in text.lines() {
            store.orchestrator.push_line(line.to_string());
        }
        return true;
    }
    false
}

fn apply_tool_start(store: &mut Store, event: &serde_json::Map<String, serde_json::Value>) -> bool {
    let tool_name = event.get("toolName").and_then(serde_json::Value::as_str).unwrap_or("tool");
    let intent = event.get("intent").and_then(serde_json::Value::as_str).unwrap_or("");
    let line = if intent.is_empty() {
        format!("[tool] {tool_name}")
    } else {
        format!("[tool] {tool_name} — {intent}")
    };
    store.orchestrator.push_line(line);
    true
}

fn apply_tool_end(store: &mut Store, event: &serde_json::Map<String, serde_json::Value>) -> bool {
    let tool_name = event.get("toolName").and_then(serde_json::Value::as_str).unwrap_or("tool");
    if tool_name == "task" {
        if let Some(result) = event.get("result").and_then(serde_json::Value::as_str) {
            let summary: String = result.lines().take(3).collect::<Vec<&str>>().join(" | ");
            store.orchestrator.push_line(format!("[tool] task done — {summary}"));
            return true;
        }
    }
    false
}

fn apply_subagent_lifecycle(store: &mut Store, frame: &omprpc::frame::SubagentLifecycleFrame) -> bool {
    let payload = &frame.payload;
    let Some(slot) = store.worker_slot_for(&payload.id) else {
        return false;
    };
    let status = payload.status.as_str();
    let line = match status {
        "started" => {
            let task = payload.description.as_deref().unwrap_or("spawned");
            format!("[spawn] {task}")
        },
        "completed" => "[spawn] completed".to_string(),
        "failed" => "[spawn] failed".to_string(),
        "aborted" => "[spawn] aborted".to_string(),
        other => format!("[spawn] {other}"),
    };
    let pane = store.pane_mut(slot);
    pane.agent_id = Some(payload.id.clone());
    pane.status = status.to_string();
    pane.push_line(line);
    true
}

fn apply_subagent_progress(store: &mut Store, frame: &omprpc::frame::SubagentProgressFrame) -> bool {
    let payload = &frame.payload;
    let Some(slot) = store.worker_slot_for(&payload.agent) else {
        return false;
    };
    let progress = &payload.progress;
    let pane = store.pane_mut(slot);
    if let Some(status) = progress.status.as_deref() {
        pane.status = status.to_string();
    }
    pane.tokens = progress.tokens;
    pane.cost = progress.cost;
    let activity = progress.last_intent.as_deref().map(str::to_string).or_else(|| progress.current_tool.clone());
    if let Some(activity) = activity {
        if pane.current_activity != activity {
            pane.current_activity = activity.clone();
            pane.push_line(format!("[activity] {activity}"));
        }
    }
    for line in &progress.recent_output {
        if !line.trim().is_empty() {
            pane.push_line(line.clone());
        }
    }
    true
}

fn apply_subagent_event(store: &mut Store, id: &str, event: &serde_json::Map<String, serde_json::Value>) -> bool {
    let Some(slot) = store.worker_slot_for(id) else {
        return false;
    };
    let event_type = event.get("type").and_then(serde_json::Value::as_str);
    match event_type {
        Some("tool_execution_start") => {
            let tool_name = event.get("toolName").and_then(serde_json::Value::as_str).unwrap_or("tool");
            let pane = store.pane_mut(slot);
            pane.push_line(format!("[tool] {tool_name}"));
            true
        },
        Some("message_update") | Some("message_end") => {
            let Some(assistant_event) = event.get("assistantMessageEvent") else {
                return false;
            };
            if assistant_event.get("type").and_then(serde_json::Value::as_str) == Some("text_delta") {
                if let Some(delta) = assistant_event.get("delta").and_then(serde_json::Value::as_str) {
                    let pane = store.pane_mut(slot);
                    for line in delta.lines() {
                        if !line.trim().is_empty() {
                            pane.push_line(line.to_string());
                        }
                    }
                    return true;
                }
            }
            false
        },
        _ => false,
    }
}

fn apply_irc_message(store: &mut Store, event: &serde_json::Map<String, serde_json::Value>) -> bool {
    let Some(message) = event.get("message") else {
        return false;
    };
    let from = message.get("from").and_then(serde_json::Value::as_str).unwrap_or("unknown");
    let body = message.get("body").and_then(serde_json::Value::as_str).unwrap_or("");
    if let Some(slot) = store.worker_slot_for(from) {
        let pane = store.pane_mut(slot);
        pane.push_line(format!("[irc from orchestrator] {body}"));
        return true;
    }
    store.orchestrator.push_line(format!("[irc from {from}] {body}"));
    true
}
