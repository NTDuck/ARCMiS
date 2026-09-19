//! No-op hook shared by inner agents. The outer hook observes only the
//! top-level agent of its method.

/// Hook that observes nothing. Inner workers run under it, so one outer
/// hook stays the single observation point of a run.
#[derive(Clone, Default)]
pub struct NoopHook;

impl rig::agent::AgentHook for NoopHook {
    async fn on_completion_call(
        &self,
        _ctx: &rig::agent::HookContext,
        _event: rig::agent::CompletionCallEvent<'_>,
    ) -> rig::agent::CompletionCallAction {
        rig::agent::CompletionCallAction::continue_run()
    }

    async fn on_tool_call(
        &self,
        _ctx: &rig::agent::HookContext,
        _event: rig::agent::ToolCall<'_>,
    ) -> rig::agent::ToolCallAction {
        rig::agent::ToolCallAction::run()
    }
}
