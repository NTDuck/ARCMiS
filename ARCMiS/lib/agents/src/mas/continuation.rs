//! Ollama continuation hook. Ollama's chat template (qwen3.8) rejects a
//! request whose message list ends with a tool result and carries no trailing
//! user query: HTTP 500 `no user query found in messages`. rig 0.42 sends
//! exactly that shape on every multi-turn tool continuation — the next
//! prompt after tool execution is the tool-result user message itself. Each
//! rejected call burns a provider retry (~15-55 s), which stalled the amp
//! benchmark cell for an hour. The hook appends a synthetic user message
//! after the tool results so the provider sees a user query.

use rig::agent::hook::{
    AgentHook, CompletionCall, CompletionCallAction, HookContext, RequestPatch,
};
use rig::message::{Message, UserContent};

/// Patch continuation requests so the history ends with a user message.
#[derive(Clone, Copy, Default)]
pub struct TrailingUserMessageHook;

impl TrailingUserMessageHook {
    /// True when `message` is a user message whose content is all tool
    /// results (the continuation shape ollama rejects).
    fn is_tool_result_message(message: Option<&Message>) -> bool {
        matches!(message, Some(Message::User { content }) if content.iter().all(|item| matches!(item, UserContent::ToolResult(_))))
    }
}

impl AgentHook for TrailingUserMessageHook {
    async fn on_completion_call(
        &self,
        _ctx: &HookContext,
        event: CompletionCall<'_>,
    ) -> CompletionCallAction {
        // The effective last message of the request is the turn prompt when
        // it stands alone, else the final history entry. Continuations carry
        // the tool-result message as the prompt; the first call carries the
        // user's chat text and needs no patch.
        let ends_with_tool_results = Self::is_tool_result_message(Some(event.prompt))
            || Self::is_tool_result_message(event.history.last());
        if !ends_with_tool_results {
            return CompletionCallAction::continue_run();
        }
        let mut patched = event.history.to_vec();
        patched.push(Message::user(
            "Tool results above. Continue the task from here.",
        ));
        CompletionCallAction::patch(RequestPatch::new().history(patched))
    }
}
