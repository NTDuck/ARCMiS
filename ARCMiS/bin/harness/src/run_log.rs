//! The run log and tracing install.
//!
//! The harness streams every agent run through one hook. Tool calls
//! surface through the rig hook with a cut-down args preview. The
//! structured results land in the tracing log, the result yaml, and a
//! JSONL trace file for the experiment history.

use agents::ValidatorStepOutcome;
use rig::agent::{
    AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, ToolCall, ToolCallAction, ToolResultAction,
    ToolResultEvent,
};
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Tool-call args preview length. Longer args are cut and marked.
const ARG_PREVIEW_CHARS: usize = 200;
/// Tool-result preview length. Longer results are cut and marked.
const RESULT_PREVIEW_CHARS: usize = 2000;

/// One trace line. Kind discriminates model calls, tool calls, and tool
/// results. Timestamps are RFC 3339 UTC wall-clock strings.
#[derive(Serialize)]
struct TraceLine<'a> {
    kind: &'a str,
    turn: usize,
    agent: Option<&'a str>,
    tool: Option<&'a str>,
    args: Option<&'a str>,
    result: Option<&'a str>,
}

/// The append-only JSONL sink for one run.
pub struct TraceSink {
    file: Mutex<Option<std::fs::File>>,
    path: PathBuf,
}

impl TraceSink {
    /// Create the sink. The file opens lazily on the first line, so an
    /// experiment dir that never writes stays empty.
    pub fn new(path: PathBuf) -> Self {
        Self {
            file: Mutex::new(None),
            path,
        }
    }

    /// Append one line. A write failure logs once and drops the line. A
    /// trace gap never fails a migration run.
    fn append(&self, line: &TraceLine<'_>) {
        let mut guard = match self.file.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if guard.is_none() {
            if let Some(parent) = self.path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            *guard = OpenOptions::new().create(true).append(true).open(&self.path).ok();
        }
        if let Some(file) = guard.as_mut() {
            let json = match serde_json::to_string(line) {
                Ok(json) => json,
                Err(_) => return,
            };
            let _ = writeln!(file, "{json}");
        }
    }
}

/// The tracing subscriber. One global install with an env filter.
pub struct Tracing;

impl Tracing {
    /// Install the subscriber. `RUST_LOG` selects the filter. `info` is the default.
    pub fn init() {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    }
}

/// The run log. Tool calls surface through the rig hook. Every model
/// call, tool call, and tool result lands in the JSONL trace sink. The
/// agents' structured results land in the tracing log and the result
/// yaml.
#[derive(Clone)]
pub struct RunLog {
    sink: SinkHandle,
}

/// Cheap cloneable share over one trace sink.
#[derive(Clone, Default)]
struct SinkHandle(Option<Arc<TraceSink>>);

impl RunLog {
    /// Build a run log that writes its trace under `trace_path`.
    pub fn with_trace(trace_path: PathBuf) -> Self {
        Self {
            sink: SinkHandle(Some(Arc::new(TraceSink::new(trace_path)))),
        }
    }
}

impl Default for RunLog {
    fn default() -> Self {
        Self {
            sink: SinkHandle(None),
        }
    }
}

impl AgentHook for RunLog {
    async fn on_completion_call(&self, ctx: &HookContext, _event: CompletionCallEvent<'_>) -> CompletionCallAction {
        tracing::info!(turn = ctx.turn(), "model call");
        if let Some(sink) = self.sink.0.as_ref() {
            sink.append(&TraceLine {
                kind: "model_call",
                turn: ctx.turn(),
                agent: ctx.agent_name(),
                tool: None,
                args: None,
                result: None,
            });
        }
        CompletionCallAction::continue_run()
    }

    async fn on_tool_call(&self, ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        tracing::info!(
            turn = ctx.turn(),
            tool = event.tool_name,
            args = %truncate_args(event.args),
            "tool call"
        );
        if let Some(sink) = self.sink.0.as_ref() {
            sink.append(&TraceLine {
                kind: "tool_call",
                turn: ctx.turn(),
                agent: ctx.agent_name(),
                tool: Some(event.tool_name),
                args: Some(event.args),
                result: None,
            });
        }
        ToolCallAction::run()
    }

    async fn on_tool_result(&self, ctx: &HookContext, event: ToolResultEvent<'_>) -> ToolResultAction {
        if let Some(sink) = self.sink.0.as_ref() {
            let rendered = match event.raw_result.output().as_text() {
                Some(text) => truncate_result(text),
                None => truncate_result(&format!("{:?}", event.raw_result.output())),
            };
            sink.append(&TraceLine {
                kind: "tool_result",
                turn: ctx.turn(),
                agent: ctx.agent_name(),
                tool: Some(event.tool_name),
                args: Some(event.args),
                result: Some(&rendered),
            });
        }
        ToolResultAction::keep()
    }
}

/// Cut `args` to [`ARG_PREVIEW_CHARS`] characters and append `...` when cut.
fn truncate_args(args: &str) -> String {
    if args.chars().count() <= ARG_PREVIEW_CHARS {
        return args.to_owned();
    }
    let cut: String = args.chars().take(ARG_PREVIEW_CHARS).collect();
    format!("{cut}...")
}

/// Cut `text` to [`RESULT_PREVIEW_CHARS`] characters and mark the cut.
fn truncate_result(text: &str) -> String {
    if text.chars().count() <= RESULT_PREVIEW_CHARS {
        return text.to_owned();
    }
    let cut: String = text.chars().take(RESULT_PREVIEW_CHARS).collect();
    format!("{cut}...")
}

/// Render one toolchain step outcome as a detail line. Test counts appear
/// when the step reported them.
pub fn step_report_line(step: &ValidatorStepOutcome) -> String {
    match (step.tests_passed, step.tests_failed) {
        (Some(passed), Some(failed)) => format!("tests passed {passed}, failed {failed}"),
        _ => String::new(),
    }
}

/// Ensure the parent directory of `path` exists before first write.
pub fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}
