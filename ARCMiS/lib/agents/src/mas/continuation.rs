//! Ollama continuation hook. Ollama's qwen3.8 chat templates reject a request
//! that carries no user query: HTTP 500 `no user query found in messages`
//! (reproduced on ollama 0.34 against both `qwen3.8:27b-mtp-q4_K_M` and
//! `smtek/Swift-Qwen3.8-27B:map-k4v` for every continuation shape with
//! tool results and no user message anywhere in the list). rig 0.42 sends
//! exactly that shape on every multi-turn tool continuation: the tool results
//! become the turn prompt (`AgentRun::tool_results` pushes them as the last
//! new message, `next_step` splits them off as the prompt), so a delegation
//! that starts from tool traffic reaches the provider with no user query.
//! Each rejected call burns a provider retry and killed whole delegations in
//! the 20260927 baseline traces (23 of 28 analyst calls in
//! `20260927T123000Z-base-crust-libm17` carried the empty-prompt shape; the
//! failure ledger records the 500s). The hook appends a synthetic user
//! message to the history so the provider sees a user query.
//!
//! The guarantee must survive history-replacing hooks registered later: the
//! snapcompact compaction hook patches the history on long turns and would
//! silently drop the synthetic message, re-triggering the 500 exactly when
//! the context grows past the compaction threshold. Both hooks share
//! [`CONTINUATION_QUERY`]; see `SnapcompactHook::trailing_user_query`.

use rig::agent::hook::AgentHook;
use rig::agent::hook::CompletionCall;
use rig::agent::hook::CompletionCallAction;
use rig::agent::hook::HookContext;
use rig::agent::hook::RequestPatch;
use rig::message::Message;
use rig::message::UserContent;

/// The synthetic user query appended after tool results.
pub const CONTINUATION_QUERY: &str = "Tool results above. Continue the task from here.";

/// True when `message` is a user message whose content is all tool results
/// (the continuation shape ollama rejects).
#[must_use]
pub fn is_tool_result_message(message: &Message) -> bool {
    matches!(message, Message::User { content } if content.iter().all(|item| matches!(item, UserContent::ToolResult(_))))
}

/// The history patch that repairs a tool-result continuation: the original
/// history plus one synthetic user query. `None` when the request needs no
/// repair — the prompt itself is a user query, and the wire always ends with
/// the prompt (history patches replace the history only; the prompt is
/// appended after it at the request boundary).
#[must_use]
pub fn continuation_patch(prompt: &Message, history: &[Message]) -> Option<RequestPatch> {
    if !is_tool_result_message(prompt) {
        return None;
    }
    let mut patched = history.to_vec();
    patched.push(Message::user(CONTINUATION_QUERY));
    Some(RequestPatch::new().history(patched))
}

/// Patch continuation requests so the request carries a user query.
#[derive(Clone, Copy, Default)]
pub struct TrailingUserMessageHook;

impl AgentHook for TrailingUserMessageHook {
    async fn on_completion_call(&self, _ctx: &HookContext, event: CompletionCall<'_>) -> CompletionCallAction {
        match continuation_patch(event.prompt, event.history) {
            Some(patch) => CompletionCallAction::patch(patch),
            None => CompletionCallAction::continue_run(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_result_prompt_yields_synthetic_user_query() {
        let prompt = Message::tool_result("t1", "read", "directory listing");
        let history = vec![Message::user("Do the task."), Message::assistant("Reading files.")];
        let patch = continuation_patch(&prompt, &history).expect("continuation needs repair");
        let patched = patch.history.expect("history patch");
        assert_eq!(patched.len(), history.len() + 1);
        assert!(
            matches!(&patched.last(), Some(Message::User { content })
            if content.iter().any(|item| matches!(item, UserContent::Text(text) if text.text == CONTINUATION_QUERY))),
            "patched history must end with the synthetic user query"
        );
    }

    #[test]
    fn user_text_prompt_needs_no_repair() {
        // Even when the history itself ends with tool results: the wire ends
        // with the prompt, which is a user query.
        let prompt = Message::user("Continue from the results above.");
        let history = vec![Message::tool_result("t1", "read", "listing")];
        assert!(continuation_patch(&prompt, &history).is_none());
    }

    #[test]
    fn tool_result_message_detection() {
        assert!(is_tool_result_message(&Message::tool_result("t1", "read", "out")));
        assert!(!is_tool_result_message(&Message::user("plain query")));
        assert!(!is_tool_result_message(&Message::assistant("answer")));
    }
}
