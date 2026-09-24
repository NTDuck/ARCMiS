//! Serialize one message history into a compact text transcript for bitmap
//! archival. Mirrors oh-my-pi's `serialize.ts`: truncate tool results
//! head+tail, cap tool-call argument values, mark tool output distinctly.

use rig::message::AssistantContent;
use rig::message::Message;
use rig::message::UserContent;

/// Options for [`serialize_history`].
#[derive(Debug, Clone)]
pub struct SerializeOptions {
    /// Characters kept per truncated tool result: head ratio of this total.
    pub tool_result_truncate: usize,
    /// Fraction of the truncate budget that goes to the head (rest to tail).
    pub head_ratio: f32,
    /// Characters kept per tool-call argument value.
    pub arg_value_cap: usize,
    /// Characters kept for one whole tool-call argument object.
    pub arg_total_cap: usize,
}

impl Default for SerializeOptions {
    fn default() -> Self {
        Self {
            tool_result_truncate: 2000,
            head_ratio: 0.6,
            arg_value_cap: 500,
            arg_total_cap: 2000,
        }
    }
}

/// Serialize a message history into one dense text transcript.
///
/// Every message becomes one line block: `USER:`, `ASSISTANT:`,
/// `TOOL_RESULT:` prefixes, tool output dim-marked `[tool-output dim]`,
/// truncation applied head+tail with an elision marker.
#[must_use]
pub fn serialize_history(messages: &[Message], opts: &SerializeOptions) -> String {
    let mut out = String::new();
    for message in messages {
        match message {
            Message::System { content, .. } => {
                out.push_str("SYSTEM: ");
                out.push_str(&one_line(content));
                out.push('\n');
            },
            Message::User { content } => {
                for item in content {
                    match item {
                        UserContent::Text(text) => {
                            out.push_str("USER: ");
                            out.push_str(&one_line(&text.text));
                            out.push('\n');
                        },
                        UserContent::ToolResult(result) => {
                            let text = tool_result_text(result);
                            out.push_str(&format!(
                                "TOOL_RESULT [tool-output dim]: {}\n",
                                truncate(&text, opts.tool_result_truncate, opts.head_ratio)
                            ));
                        },
                        other => {
                            out.push_str(&format!("USER [other]: {other:?}\n"));
                        },
                    }
                }
            },
            Message::Assistant { content, .. } => {
                for item in content {
                    match item {
                        AssistantContent::Text(text) => {
                            out.push_str("ASSISTANT: ");
                            out.push_str(&one_line(&text.text));
                            out.push('\n');
                        },
                        AssistantContent::ToolCall(call) => {
                            out.push_str(&format!(
                                "TOOL_CALL {}: {}\n",
                                call.function.name,
                                truncate(
                                    &call.function.arguments.to_string(),
                                    opts.arg_total_cap,
                                    opts.head_ratio
                                )
                            ));
                        },
                        AssistantContent::Reasoning(reasoning) => {
                            let text = reasoning
                                .content
                                .iter()
                                .map(|block| match block {
                                    rig::message::ReasoningContent::Text { text, .. } => text.as_str(),
                                    _ => "",
                                })
                                .collect::<Vec<_>>()
                                .join(" ");
                            out.push_str("THINKING: ");
                            out.push_str(&one_line(&text));
                            out.push('\n');
                        },
                        AssistantContent::Image(_) => {
                            out.push_str("ASSISTANT: [image]\n");
                        },
                    }
                }
            },
        }
    }
    out
}

/// Collapse all whitespace to single spaces so lines stay dense.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extract the text of one tool result block.
fn tool_result_text(result: &rig::message::ToolResult) -> String {
    result
        .content
        .iter()
        .map(|block| match block {
            rig::message::ToolResultContent::Text(text) => text.text.clone(),
            rig::message::ToolResultContent::Image(_) => "[image]".to_owned(),
            rig::message::ToolResultContent::Json { .. } => "[json]".to_owned(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Truncate `text` to `total` chars, keeping the head at `head_ratio` and the
/// tail at the remainder, with an elision marker between.
fn truncate(text: &str, total: usize, head_ratio: f32) -> String {
    let total = total.max(16);
    if text.chars().count() <= total {
        return one_line(text);
    }
    let head = ((total as f32) * head_ratio) as usize;
    let tail = total.saturating_sub(head).max(8);
    let chars: Vec<char> = text.chars().collect();
    let head_text: String = chars[..head].iter().collect();
    let tail_text: String = chars[chars.len() - tail..].iter().collect();
    format!("{} ...[elided]... {}", one_line(&head_text), one_line(&tail_text))
}

/// Rough token estimate: 4 bytes per token (oh-my-pi's cheap proxy).
#[must_use]
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() as u32 / 4).max(1)
}
