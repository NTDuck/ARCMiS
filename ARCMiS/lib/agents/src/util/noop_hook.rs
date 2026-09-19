//! No-op hook shared by inner agents. The outer hook observes only the
//! top-level agent of its method.

use rig::agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, ToolCall, ToolCallAction};

/// Hook that observes nothing. Inner workers run under it, so one outer
/// hook stays the single observation point of a run.
#[derive(Clone, Default)]
pub struct NoopHook;

impl AgentHook for NoopHook {
    async fn on_completion_call(&self, _ctx: &HookContext, _event: CompletionCallEvent<'_>) -> CompletionCallAction {
        CompletionCallAction::continue_run()
    }

    async fn on_tool_call(&self, _ctx: &HookContext, _event: ToolCall<'_>) -> ToolCallAction {
        ToolCallAction::run()
    }
}
