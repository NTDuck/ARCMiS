//! Per-turn trace hook. One JSONL line per model call, model response, tool
//! call, and tool result at `{output_dir}/traces/turns.jsonl`. The hook is
//! observe-only: every callback returns `Continue`.

use rig::agent::hook::{AgentHook, CompletionCall, CompletionCallAction, CompletionResponse,
    HookContext, ObservationAction, ToolCall, ToolCallAction, ToolResultAction,
    ToolResultEvent};
use serde_json::Value;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Append-only per-turn trace sink.
pub struct TraceSink {
    path: PathBuf,
    writer: Mutex<()>,
}

impl TraceSink {
    /// Bind the sink at `{dir}/traces/turns.jsonl`.
    pub fn new(dir: &Path) -> anyhow::Result<Self> {
        let dir = dir.join("traces");
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            path: dir.join("turns.jsonl"),
            writer: Mutex::new(()),
        })
    }

    /// Append one JSONL record.
    fn append(&self, event: &str, mut fields: Value) {
        if let Value::Object(map) = &mut fields {
            if let Some(text) = map.get_mut("text") {
                cap_string(text, 2000);
            }
            if let Some(args) = map.get_mut("args") {
                cap_string(args, 2000);
            }
            if let Some(result) = map.get_mut("result") {
                cap_string(result, 2000);
            }
        }
        let unix_seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let record = serde_json::json!({
            "at": unix_seconds,
            "event": event,
            "fields": fields,
        });
        let line = record.to_string();
        let _guard = self.writer.lock();
        let Ok(mut handle) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        else {
            return;
        };
        let _ = handle.write_all(line.as_bytes());
        let _ = handle.write_all(b"\n");
        let _ = handle.flush();
    }
}

/// Cap a JSON string value in place.
fn cap_string(value: &mut Value, cap: usize) {
    if let Value::String(text) = value {
        if text.len() > cap {
            let mut end = cap;
            while end > 0 && !text.is_char_boundary(end) {
                end -= 1;
            }
            *text = format!("{}…[{}B truncated]", &text[..end], text.len() - end);
        }
    }
}

/// Which named agent emitted the events; set at build time.
pub struct TraceHook {
    agent: &'static str,
    sink: std::sync::Arc<TraceSink>,
}

impl TraceHook {
    /// Attach the hook for one named agent.
    pub fn new(agent: &'static str, sink: std::sync::Arc<TraceSink>) -> Self {
        Self { agent, sink }
    }
}

impl AgentHook for TraceHook {
    async fn on_completion_call(
        &self,
        _ctx: &HookContext,
        event: CompletionCall<'_>,
    ) -> CompletionCallAction {
        self.sink.append(
            "model_call",
            serde_json::json!({
                "agent": self.agent,
                "prompt": event.prompt.rag_text().unwrap_or_default(),
            }),
        );
        CompletionCallAction::Continue
    }

    async fn on_completion_response(
        &self,
        _ctx: &HookContext,
        event: CompletionResponse<'_>,
    ) -> ObservationAction {
        self.sink.append(
            "model_response",
            serde_json::json!({
                "agent": self.agent,
                "usage": {
                    "input_tokens": event.usage.input_tokens,
                    "output_tokens": event.usage.output_tokens,
                    "total_tokens": event.usage.total_tokens,
                    "cached_input_tokens": event.usage.cached_input_tokens,
                },
                "text": event
                    .content
                    .iter()
                    .filter_map(|part| match part {
                        rig::completion::AssistantContent::Text(text) => {
                            Some(text.text.clone())
                        },
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            }),
        );
        ObservationAction::Continue
    }

    async fn on_tool_call(&self, _ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        self.sink.append(
            "tool_call",
            serde_json::json!({
                "agent": self.agent,
                "tool": event.tool_name,
                "args": event.args,
            }),
        );
        ToolCallAction::run()
    }

    async fn on_tool_result(
        &self,
        _ctx: &HookContext,
        event: ToolResultEvent<'_>,
    ) -> ToolResultAction {
        self.sink.append(
            "tool_result",
            serde_json::json!({
                "agent": self.agent,
                "tool": event.tool_name,
                "result": format!("{:?}", event.raw_result),
            }),
        );
        ToolResultAction::Keep
    }
}

/// Open the trace file lazily on append; a failed open drops the record.
pub fn sink(path: &Path) -> anyhow::Result<std::sync::Arc<TraceSink>> {
    Ok(std::sync::Arc::new(TraceSink::new(path)?))
}
