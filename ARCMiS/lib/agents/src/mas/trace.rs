//! Per-turn trace hook. One JSONL line per model call, model response, tool
//! call, and tool result at `{output_dir}/traces/turns.jsonl`. The hook is
//! observe-only: every callback returns `Continue`.

use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use rig::agent::hook::AgentHook;
use rig::agent::hook::CompletionCall;
use rig::agent::hook::CompletionCallAction;
use rig::agent::hook::CompletionResponse;
use rig::agent::hook::HookContext;
use rig::agent::hook::ObservationAction;
use rig::agent::hook::ToolCall;
use rig::agent::hook::ToolCallAction;
use rig::agent::hook::ToolResultAction;
use rig::agent::hook::ToolResultEvent;
use serde_json::Value;

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
            if let Some(reason) = map.get_mut("skip_reason") {
                cap_string(reason, 2000);
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
        let Ok(mut handle) = std::fs::OpenOptions::new().create(true).append(true).open(&self.path) else {
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
    sink: Arc<TraceSink>,
}

impl TraceHook {
    /// Attach the hook for one named agent.
    pub fn new(agent: &'static str, sink: Arc<TraceSink>) -> Self {
        Self {
            agent,
            sink,
        }
    }
}

impl AgentHook for TraceHook {
    async fn on_completion_call(&self, _ctx: &HookContext, event: CompletionCall<'_>) -> CompletionCallAction {
        self.sink.append(
            "model_call",
            serde_json::json!({
                "agent": self.agent,
                "prompt": event.prompt.rag_text().unwrap_or_default(),
            }),
        );
        CompletionCallAction::Continue
    }

    async fn on_completion_response(&self, _ctx: &HookContext, event: CompletionResponse<'_>) -> ObservationAction {
        self.sink.append("model_response", response_fields(self.agent, event));
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

    async fn on_tool_result(&self, _ctx: &HookContext, event: ToolResultEvent<'_>) -> ToolResultAction {
        self.sink.append("tool_result", result_fields(self.agent, event.tool_name, event.raw_result));
        ToolResultAction::Keep
    }
}

/// Trace fields for one tool result. Status plus reason: a skipped result
/// (gate denial, budget stop) carries its model feedback in the output text;
/// the Debug form used to drop it, and triage miscounted gate denials as a
/// dead hook.
fn result_fields(agent: &str, tool: &str, result: &rig::tool::ToolResult) -> Value {
    let mut fields = serde_json::json!({
        "agent": agent,
        "tool": tool,
        "status": result.status_name(),
    });
    if result.is_skipped() {
        if let Some(reason) = result.output().as_text() {
            fields["skip_reason"] = serde_json::Value::String(reason.to_owned());
        }
    } else {
        fields["result"] = serde_json::Value::String(format!("{result:?}"));
    }
    fields
}

/// Open the trace file lazily on append; a failed open drops the record.
pub fn sink(path: &Path) -> anyhow::Result<Arc<TraceSink>> {
    Ok(Arc::new(TraceSink::new(path)?))
}

/// Trace fields for one model response. Tool-call names ride alongside the
/// text so a tool-only turn (empty text, executed calls) is not misread as a
/// missing answer.
fn response_fields(agent: &str, event: CompletionResponse<'_>) -> Value {
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();
    for part in event.content {
        match part {
            rig::completion::AssistantContent::Text(text) => text_parts.push(text.text.clone()),
            rig::completion::AssistantContent::ToolCall(call) => tool_calls.push(call.function.name.clone()),
            _ => {},
        }
    }
    serde_json::json!({
        "agent": agent,
        "usage": {
            "input_tokens": event.usage.input_tokens,
            "output_tokens": event.usage.output_tokens,
            "total_tokens": event.usage.total_tokens,
            "cached_input_tokens": event.usage.cached_input_tokens,
        },
        "text": text_parts.join("\n"),
        "tool_calls": tool_calls,
    })
}

#[cfg(test)]
mod tests {
    use rig::completion::AssistantContent;
    use rig::message::Text;
    use rig::message::ToolCall;
    use rig::message::ToolCallId;
    use rig::message::ToolFunction;

    use super::*;

    #[test]
    fn response_with_only_tool_calls_records_the_calls() {
        // Regression: a tool-only turn used to log `text: ""` with no trace
        // of the calls, and every triage pass misread it as a model failure.
        let content = vec![
            AssistantContent::Text(Text::new(String::new())),
            AssistantContent::ToolCall(ToolCall::new(
                ToolCallId::new("test-call-1").unwrap(),
                ToolFunction::new("read_file".into(), serde_json::json!({"path": "src/main.rs"})),
            )),
        ];
        let response = CompletionResponse {
            prompt: &rig::completion::Message::User {
                content: Vec::new(),
            },
            content: &content,
            usage: Default::default(),
            message_id: None,
            identity: &Default::default(),
            raw: &serde_json::Value::Null,
        };
        let fields = response_fields("analyst", response);
        let calls = fields["tool_calls"].as_array().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "read_file");
        assert_eq!(fields["text"], "");
    }

    #[test]
    fn skipped_tool_result_records_the_reason() {
        // Regression: a gate-skipped read used to log only the Debug status
        // (`status: "skipped"`), dropping the denial reason - cycle-6 triage
        // then counted zero gate trips where the trace held ~10.
        let result = rig::tool::ToolResult::skipped("read would exhaust the context window");
        let fields = result_fields("analyst", "read", &result);
        assert_eq!(fields["status"], "skipped");
        assert_eq!(fields["skip_reason"], "read would exhaust the context window");
        assert!(fields.get("result").is_none());
    }

    #[test]
    fn successful_tool_result_keeps_the_debug_form() {
        let result = rig::tool::ToolResult::success(rig::tool::ToolOutput::text("ok"));
        let fields = result_fields("analyst", "read", &result);
        assert_eq!(fields["status"], "success");
        assert!(fields["result"].as_str().is_some());
        assert!(fields.get("skip_reason").is_none());
    }
}
