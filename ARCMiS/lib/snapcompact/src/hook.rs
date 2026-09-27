//! Snapcompact hook: attaches archived bitmap frames to the next model call
//! when the projected input exceeds the threshold.

use bdf_parser::BdfFont;
use rig::agent::hook::CompletionCall;
use rig::agent::AgentHook;
use rig::agent::CompletionCallAction;
use rig::agent::HookContext;
use rig::completion::Document;
use rig::completion::Message;

use crate::compact::compact;
use crate::compact::CompactOptions;

/// Hook that compacts a long history before a completion call.
///
/// The hook reads the pending history from the completion-call event
/// (`CompletionCall.history`), so no separate history tracking is needed:
/// rig hands the hook the exact messages the next request would carry.
#[derive(Clone)]
pub struct SnapcompactHook {
    /// Compact only when the projected input exceeds this token count.
    pub threshold_tokens: u32,
    /// Compaction knobs.
    pub options: CompactOptions,
    /// Shared font, parsed once at startup.
    pub font: std::sync::Arc<BdfFont>,
    /// User query appended after the archived frames when the turn's prompt
    /// is a tool-result-only message. The hook's history patch replaces any
    /// history patch an earlier hook installed — including a trailing-user
    /// repair — and the prompt the provider appends after the history is
    /// itself tool results on those turns, so without this the compacted
    /// request reaches ollama with no user query and dies with HTTP 500
    /// `no user query found in messages`. Set it to the same synthetic query
    /// the continuation hook uses so the guarantee survives compaction.
    pub trailing_user_query: Option<String>,
}

impl SnapcompactHook {
    /// The history patch this hook installs for one completion call, or
    /// `None` when the projected input is under the threshold or nothing
    /// was archived. Testable without a rig run: the hook method delegates
    /// here.
    pub(crate) fn patch_for(&self, prompt: &Message, history: &[Message]) -> Option<rig::agent::RequestPatch> {
        let projected: u32 =
            std::iter::once(prompt).chain(history.iter()).map(crate::compact::message_tokens_public).sum();
        if projected <= self.threshold_tokens {
            return None;
        }
        let mut messages: Vec<Message> = history.to_vec();
        messages.push(prompt.clone());
        let result = compact(messages, &self.font, &self.options);
        if result.frames.is_empty() {
            return None;
        }
        let pngs = crate::render::encode_png(&result.frames);
        let images: Vec<rig::message::UserContent> = pngs
            .iter()
            .map(|bytes| {
                rig::message::UserContent::Image(rig::message::Image {
                    data: rig::message::DocumentSourceKind::base64(use_base64_encode(bytes).as_str()),
                    media_type: Some(rig::message::ImageMediaType::PNG),
                    detail: None,
                    additional_params: None,
                })
            })
            .collect();
        // A tool-result-only prompt means the request ends with tool results
        // (the provider appends the prompt after the history), so the compacted
        // history must carry a user query itself or ollama rejects the whole
        // request. Append the caller's query after the frames.
        let trailing = self
            .trailing_user_query
            .as_deref()
            .filter(|_| is_tool_result_prompt(prompt))
            .map(rig::completion::Message::user);
        let history_patch = [result.kept, vec![Message::User {
            content: images,
        }]]
        .concat()
        .into_iter()
        .chain(trailing)
        .collect::<Vec<_>>();
        tracing::info!(
            tokens_before = result.tokens_before,
            tokens_after = result.tokens_after,
            frames = result.frames.len(),
            "snapcompact archived older history to PNG frames"
        );
        Some(rig::agent::RequestPatch::new().history(history_patch).extra_context([Document {
            id: "snapcompact-summary".to_owned(),
            text: result.summary_text.clone(),
            additional_props: Default::default(),
        }]))
    }
}

/// True when `message` is a user message whose content is all tool results —
/// the continuation shape whose request the provider ends with tool output.
fn is_tool_result_prompt(message: &Message) -> bool {
    matches!(message, Message::User { content } if content.iter().all(|item| matches!(item, rig::message::UserContent::ToolResult(_))))
}

impl AgentHook for SnapcompactHook {
    async fn on_completion_call(&self, _ctx: &HookContext, event: CompletionCall<'_>) -> CompletionCallAction {
        match self.patch_for(event.prompt, event.history) {
            Some(patch) => CompletionCallAction::patch(patch),
            None => CompletionCallAction::continue_run(),
        }
    }
}

/// Hex-free base64: the model wire expects standard base64.
fn use_base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let triple = [chunk[0], chunk.get(1).copied().unwrap_or(0), chunk.get(2).copied().unwrap_or(0)];
        let packed = (u32::from(triple[0]) << 16) | (u32::from(triple[1]) << 8) | u32::from(triple[2]);
        out.push(TABLE[(packed >> 18) as usize & 63] as char);
        out.push(TABLE[(packed >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(packed >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[packed as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    const QUERY: &str = "Tool results above. Continue the task from here.";

    fn hook(threshold: u32, trailing: Option<&str>) -> SnapcompactHook {
        SnapcompactHook {
            threshold_tokens: threshold,
            options: CompactOptions {
                keep_recent_tokens: 10,
                ..CompactOptions::default()
            },
            font: Arc::new(crate::load_font()),
            trailing_user_query: trailing.map(str::to_owned),
        }
    }

    fn big_history() -> Vec<Message> {
        let filler = "x".repeat(4000);
        (0..8)
            .map(|i| {
                if i % 2 == 0 {
                    Message::user(filler.clone())
                } else {
                    Message::assistant(filler.clone())
                }
            })
            .collect()
    }

    #[test]
    fn compaction_appends_trailing_user_query_on_tool_result_prompt() {
        let hook = hook(100, Some(QUERY));
        let prompt = Message::tool_result("t1", "read", "listing");
        let patch = hook.patch_for(&prompt, &big_history()).expect("over threshold: patch");
        let patched = patch.history.expect("history patch");
        let last = patched.last().expect("non-empty");
        assert!(
            matches!(last, Message::User { content }
                if content.iter().any(|item| matches!(item, rig::message::UserContent::Text(text) if text.text == QUERY))),
            "compacted history must end with the synthetic user query on a tool-result prompt; got {last:?}"
        );
    }

    #[test]
    fn compaction_without_query_option_keeps_frames_message_last() {
        // Documents the pre-fix behavior: without the trailing query the
        // compacted history ends with the frame images and the request would
        // end with tool results — the shape ollama rejects.
        let hook = hook(100, None);
        let prompt = Message::tool_result("t1", "read", "listing");
        let patch = hook.patch_for(&prompt, &big_history()).expect("over threshold: patch");
        let patched = patch.history.expect("history patch");
        assert!(
            matches!(patched.last(), Some(Message::User { content })
            if content.iter().all(|item| matches!(item, rig::message::UserContent::Image(_)))),
            "frames message must be last when no trailing query is set"
        );
    }

    #[test]
    fn compaction_skips_repair_for_user_text_prompt() {
        let hook = hook(100, Some(QUERY));
        let prompt = Message::user("Continue from the results above.");
        let patch = hook.patch_for(&prompt, &big_history()).expect("over threshold: patch");
        let patched = patch.history.expect("history patch");
        let last = patched.last().expect("non-empty");
        assert!(
            matches!(last, Message::User { content }
                if content.iter().all(|item| matches!(item, rig::message::UserContent::Image(_)))),
            "a real user prompt needs no synthetic query; got {last:?}"
        );
    }

    #[test]
    fn under_threshold_yields_no_patch() {
        let hook = hook(1_000_000, Some(QUERY));
        let prompt = Message::tool_result("t1", "read", "listing");
        assert!(hook.patch_for(&prompt, &big_history()).is_none());
    }
}
