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
use rig::message::AssistantContent;
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
/// the configured value and grows the cap once when the provider cuts a
/// turn short, so one truncated file write does not poison a phase.
#[derive(Clone)]
pub struct ResilienceHook<H> {
    inner: H,
    cap: Arc<AtomicU64>,
    ceiling: u64,
}

impl<H> ResilienceHook<H> {
    /// Wrap an inner hook (typically the run log) with the resilience
    /// policy. The cap starts at the config value and may grow to
    /// `ceiling` (four times the start) on a truncated turn.
    pub fn new(inner: H, max_output_tokens: u64) -> Self {
        Self {
            inner,
            cap: Arc::new(AtomicU64::new(max_output_tokens)),
            ceiling: max_output_tokens.saturating_mul(4),
        }
    }
}

impl<H: AgentHook + Sync> AgentHook for ResilienceHook<H> {
    async fn on_completion_call(&self, ctx: &HookContext, event: CompletionCallEvent<'_>) -> CompletionCallAction {
        // The hook prepares every attempt afresh, so the current cap
        // rides each request, grown or not.
        let cap = self.cap.load(Ordering::Relaxed);
        let inner = self.inner.on_completion_call(ctx, event).await;
        match inner {
            CompletionCallAction::Continue => CompletionCallAction::patch(RequestPatch::new().max_tokens(cap)),
            // The inner hook's own patch wins over the resilience cap:
            // rebuild one patch, inner fields applied after the cap.
            // RequestPatch::merge is private, so compose by hand.
            CompletionCallAction::Patch(inner_patch) => {
                let mut patch = RequestPatch::new().max_tokens(cap);
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
                CompletionCallAction::Patch(patch)
            },
            stop => stop,
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
