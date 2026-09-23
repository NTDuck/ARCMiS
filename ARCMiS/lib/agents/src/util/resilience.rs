//! Resilience plumbing shared by every agent run. Two layers:
//!
//! 1. The retry helper wraps one typed task with `max_retries` whole-run
//!    attempts. Transient provider failures (HTTP 500, empty responses)
//!    kill one attempt and never the whole pipeline.
//! 2. The [`ResilienceHook`] patches every model call with the
//!    configured output cap and retries a turn the provider cut short,
//!    following the upstream `retry_on_truncation` example: read
//!    `FinishReason::truncated_output`, check the tool-call guard, grow
//!    the cap once, retry with the same budget.

use rig::agent::{
    AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, ModelTurnAction, ModelTurnFinished,
    RequestPatch, ToolCall, ToolCallAction, ToolResultAction, ToolResultEvent,
};
use rig::completion::FinishReason;
use rig::message::{AssistantContent, Message};
use rig::tool::ToolOutput;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
/// Whole-attempt retry around one typed task. `max_retries` counts the
/// extra attempts after the first, matching the config field semantics.
pub async fn task_with_retry<Response>(
    mut run: impl AsyncFnMut() -> anyhow::Result<Response>,
    max_retries: u32,
) -> anyhow::Result<Response> {
    let mut last_error = None;
    for attempt in 0..=max_retries {
        if attempt > 0 {
            tracing::warn!(attempt, "retrying task after failure");
        }
        match run().await {
            Ok(response) => return Ok(response),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.expect("retry loop ran at least once"))
}

/// Hook that every top-level agent carries. It caps the output tokens at
/// the configured value, grows the cap once when the provider cuts a
/// turn short, and, on ollama, keeps a user turn at the history tail so
/// the chat template accepts the request.
#[derive(Clone)]
pub struct ResilienceHook<H> {
    inner: H,
    cap: Arc<AtomicU64>,
    ceiling: u64,
    /// Ollama rejects a chat whose messages end with an assistant
    /// message ("no user query found in messages"). Survey runs lost 17
    /// attempts to it. Only ollama sets this flag.
    needs_trailing_user: bool,
}

impl<H> ResilienceHook<H> {
    /// Wrap an inner hook (typically the run log) with the resilience
    /// policy. The cap starts at the config value and may grow to
    /// `ceiling` (four times the start) on a truncated turn. `ollama`
    /// enables the trailing-user-turn guard.
    pub fn new(inner: H, max_output_tokens: u64) -> Self {
        Self::for_provider(inner, max_output_tokens, false)
    }

    /// Provider-aware constructor. The harness passes `ollama = true`
    /// for the native ollama client.
    pub fn for_provider(inner: H, max_output_tokens: u64, ollama: bool) -> Self {
        Self {
            inner,
            cap: Arc::new(AtomicU64::new(max_output_tokens)),
            ceiling: max_output_tokens.saturating_mul(4),
            needs_trailing_user: ollama,
        }
    }

    /// Build the guarded history for one model call. When the request
    /// would end with an assistant message, append one synthetic user
    /// turn that tells the model to continue. Ollama's chat template
    /// rejects the bare-assistant tail otherwise.
    fn guarded_history(&self, history: &[Message]) -> Option<Vec<Message>> {
        if !self.needs_trailing_user {
            return None;
        }
        let ends_with_assistant = history.last().is_some_and(|message| matches!(message, Message::Assistant { .. }));
        if !ends_with_assistant {
            return None;
        }
        let mut patched = history.to_vec();
        patched.push(Message::user("Continue with the task. Use the tools and the workspace files."));
        Some(patched)
    }
}

impl<H: AgentHook + Sync> AgentHook for ResilienceHook<H> {
    async fn on_completion_call(&self, ctx: &HookContext, event: CompletionCallEvent<'_>) -> CompletionCallAction {
        // The hook prepares every attempt afresh, so the current cap
        // rides each request, grown or not.
        let cap = self.cap.load(Ordering::Relaxed);
        let inner = self.inner.on_completion_call(ctx, event).await;
        let guarded = self.guarded_history(event.history);
        let history_guard = guarded.map(|history| RequestPatch::new().history(history));
        match (history_guard, inner) {
            // The inner hook's own patch wins over the resilience fields:
            // rebuild one patch, resilience fields first, inner fields
            // applied after. RequestPatch::merge is private, so compose
            // by hand.
            (guard, CompletionCallAction::Patch(inner_patch)) => {
                let mut patch = RequestPatch::new().max_tokens(cap);
                if let Some(history) = guard.and_then(|patch| patch.history) {
                    patch = patch.history(history);
                }
                if let Some(value) = inner_patch.max_tokens {
                    patch = patch.max_tokens(value);
                }
                if let Some(value) = inner_patch.temperature {
                    patch = patch.temperature(value);
                }
                if let Some(value) = inner_patch.tool_choice.clone() {
                    patch = patch.tool_choice(value);
                }
                if let Some(value) = &inner_patch.preamble {
                    patch = patch.preamble(value.clone());
                }
                if let Some(value) = inner_patch.additional_params.clone() {
                    patch = patch.additional_params(value);
                }
                if let Some(history) = inner_patch.history {
                    patch = patch.history(history);
                }
                CompletionCallAction::Patch(patch)
            },
            (guard, inner) => {
                if let Some(history) = guard.and_then(|patch| patch.history) {
                    let patch = RequestPatch::new().max_tokens(cap).history(history);
                    return CompletionCallAction::Patch(patch);
                }
                inner
            },
        }
    }

    async fn on_model_turn_finished(&self, ctx: &HookContext, event: ModelTurnFinished<'_>) -> ModelTurnAction {
        let truncated = event.finish_reason.is_some_and(FinishReason::truncated_output);
        let has_tool_call = event.content.iter().any(|content| matches!(content, AssistantContent::ToolCall(_)));
        let room = event.max_tokens.is_none_or(|cap| cap < self.ceiling);
        if truncated && !has_tool_call && room {
            let grown = event.max_tokens.map_or(self.ceiling, |cap| cap.saturating_mul(2).min(self.ceiling));
            self.cap.store(grown, Ordering::Relaxed);
            tracing::warn!(turn = event.turn, new_cap = grown, "truncated turn retried with larger cap");
            return ModelTurnAction::repeat();
        }
        self.inner.on_model_turn_finished(ctx, event).await
    }

    async fn on_tool_call(&self, ctx: &HookContext, event: ToolCall<'_>) -> ToolCallAction {
        self.inner.on_tool_call(ctx, event).await
    }

    async fn on_tool_result(&self, ctx: &HookContext, event: ToolResultEvent<'_>) -> ToolResultAction {
        // A failed tool surfaces its raw error kind feedback ("the tool
        // failed") to the model. Rewrite the presentation with the real
        // error message so the caller can adapt instead of thrashing.
        if let Some(error) = event.raw_result.error() {
            let detail = format!("tool `{}` failed: {}", event.tool_name, error.message());
            tracing::warn!(tool = event.tool_name, "tool execution failed");
            return ToolResultAction::Rewrite(ToolOutput::text(detail));
        }
        self.inner.on_tool_result(ctx, event).await
    }
}
