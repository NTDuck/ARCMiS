//! Snapcompact hook: attaches archived bitmap frames to the next model call
//! when the projected input exceeds the threshold.

use crate::compact::compact;
use crate::compact::CompactOptions;
use bdf_parser::BdfFont;
use rig::agent::hook::CompletionCall;
use rig::agent::AgentHook;
use rig::agent::CompletionCallAction;
use rig::agent::HookContext;
use rig::completion::Document;
use rig::completion::Message;

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
}

impl AgentHook for SnapcompactHook {
    async fn on_completion_call(&self, _ctx: &HookContext, event: CompletionCall<'_>) -> CompletionCallAction {
        let projected: u32 =
            std::iter::once(event.prompt).chain(event.history.iter()).map(crate::compact::message_tokens_public).sum();
        if projected <= self.threshold_tokens {
            return CompletionCallAction::continue_run();
        }
        let mut messages: Vec<Message> = event.history.to_vec();
        messages.push(event.prompt.clone());
        let result = compact(messages, &self.font, &self.options);
        if result.frames.is_empty() {
            return CompletionCallAction::continue_run();
        }
        let pngs = crate::render::encode_png(&result.frames);
        let mut patch = rig::agent::RequestPatch::new().history(result.kept.clone()).extra_context([Document {
            id: "snapcompact-summary".to_owned(),
            text: result.summary_text.clone(),
            additional_props: Default::default(),
        }]);
        // Attach frames as base64 PNG user content via the history patch:
        // one extra user message carrying every frame.
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
        patch = patch.history(
            [
                result.kept,
                vec![Message::User {
                    content: images,
                }],
            ]
            .concat(),
        );
        tracing::info!(
            tokens_before = result.tokens_before,
            tokens_after = result.tokens_after,
            frames = result.frames.len(),
            "snapcompact archived older history to PNG frames"
        );
        CompletionCallAction::patch(patch)
    }
}

/// Hex-free base64: the model wire expects standard base64.
fn use_base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], chunk.get(1).copied().unwrap_or(0), chunk.get(2).copied().unwrap_or(0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}
