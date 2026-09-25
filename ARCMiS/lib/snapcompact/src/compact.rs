//! Compact a message history: keep recent messages verbatim, serialize the
//! rest to text, render the text to PNG frames.

use crate::render::render_frames;
use crate::serialize::estimate_tokens;
use crate::serialize::serialize_history;
use crate::serialize::SerializeOptions;
use bdf_parser::BdfFont;
use image::ImageBuffer;
use image::Rgba;
use rig::message::Message;

/// Options for [`compact`].
#[derive(Debug, Clone)]
pub struct CompactOptions {
    /// Token budget of recent messages kept verbatim.
    pub keep_recent_tokens: u32,
    /// Maximum archived frames.
    pub max_frames: u32,
    /// Archive frame width in pixels.
    pub frame_width_px: u32,
    /// Archive frame height in pixels.
    pub frame_height_px: u32,
    /// Serialization options.
    pub serialize: SerializeOptions,
}

impl Default for CompactOptions {
    fn default() -> Self {
        Self {
            keep_recent_tokens: 20_000,
            max_frames: 80,
            frame_width_px: crate::render::DEFAULT_FRAME_WIDTH,
            frame_height_px: crate::render::DEFAULT_FRAME_HEIGHT,
            serialize: SerializeOptions::default(),
        }
    }
}

/// Result of one compaction pass.
#[derive(Debug)]
pub struct CompactResult {
    /// Recent messages kept verbatim.
    pub kept: Vec<Message>,
    /// Short textual summary line placed in the next prompt.
    pub summary_text: String,
    /// Archived PNG frames covering the serialized older history.
    pub frames: Vec<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    /// Estimated tokens before compaction.
    pub tokens_before: u32,
    /// Estimated tokens after compaction (kept messages + summary only).
    pub tokens_after: u32,
}

/// Compact a history: walk backward from the newest message, keep the most
/// recent `keep_recent_tokens` verbatim, serialize the rest, render frames
/// capped at `max_frames`.
///
/// Returns the kept messages and the archive. The caller replaces the
/// history with `kept` and attaches the frames to the next model call.
#[must_use]
pub fn compact(messages: Vec<Message>, font: &BdfFont, opts: &CompactOptions) -> CompactResult {
    let tokens_before: u32 = messages.iter().map(message_tokens).sum();
    // Walk backward, accumulating messages until the budget is spent.
    let mut keep_from = messages.len();
    let mut budget = opts.keep_recent_tokens;
    for (index, message) in messages.iter().enumerate().rev() {
        let cost = message_tokens(message);
        if budget < cost && index < messages.len() {
            break;
        }
        budget = budget.saturating_sub(cost);
        keep_from = index;
    }
    let kept: Vec<Message> = messages[keep_from..].to_vec();
    let older: Vec<Message> = messages[..keep_from].to_vec();

    let tokens_after: u32 = kept.iter().map(message_tokens).sum();
    if older.is_empty() {
        return CompactResult {
            kept,
            summary_text: String::new(),
            frames: Vec::new(),
            tokens_before,
            tokens_after,
        };
    }

    let transcript = serialize_history(&older, &opts.serialize);
    let frames = render_frames(&transcript, font, opts.frame_width_px, opts.frame_height_px);
    let truncated = &frames[..frames.len().min(opts.max_frames as usize)];
    let summary_text = format!(
        "[earlier conversation archived as {} bitmap image(s); ~{} tokens elided]",
        truncated.len(),
        estimate_tokens(&transcript)
    );

    CompactResult {
        kept,
        summary_text,
        frames: truncated.to_vec(),
        tokens_before,
        tokens_after,
    }
}

/// One message's rough token cost (text bytes / 4, tool args included).
fn message_tokens(message: &Message) -> u32 {
    message_tokens_public(message)
}

/// One message's rough token cost. Public: the hook reuses the same estimate
/// for its threshold check.
#[must_use]
pub fn message_tokens_public(message: &Message) -> u32 {
    match message {
        Message::System {
            content,
            ..
        } => estimate_tokens(content),
        Message::User {
            content,
        } => content
            .iter()
            .map(|item| match item {
                rig::message::UserContent::Text(text) => estimate_tokens(&text.text),
                rig::message::UserContent::ToolResult(result) => {
                    let bytes: usize = result
                        .content
                        .iter()
                        .map(|block| match block {
                            rig::message::ToolResultContent::Text(text) => text.text.len(),
                            _ => 32,
                        })
                        .sum();
                    estimate_tokens(&String::from_utf8_lossy(&bytes.to_le_bytes()))
                },
                _ => 16,
            })
            .sum(),
        Message::Assistant {
            content,
            ..
        } => content
            .iter()
            .map(|item| match item {
                rig::message::AssistantContent::Text(text) => estimate_tokens(&text.text),
                rig::message::AssistantContent::ToolCall(call) => estimate_tokens(&call.function.arguments.to_string()),
                rig::message::AssistantContent::Reasoning(reasoning) => {
                    let bytes: usize = reasoning
                        .content
                        .iter()
                        .map(|block| match block {
                            rig::message::ReasoningContent::Text {
                                text,
                                ..
                            } => text.len(),
                            _ => 8,
                        })
                        .sum();
                    (bytes as u32 / 4).max(1)
                },
                rig::message::AssistantContent::Image(_) => 512,
            })
            .sum(),
    }
}
