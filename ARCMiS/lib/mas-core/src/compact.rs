//! `compact` is the snapcompact-style context compaction (ADR 0031).
//!
//! One context frame is a list of byte slices. The token budget
//! projection is `bytes / 4` with a floor of one, matching the retired
//! `hook.rs` threshold arithmetic exactly. Compaction walks backward
//! from the newest frame, keeps what fits, and archives the rest in
//! oldest-first order.

/// One context frame awaiting a keep-or-archive decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextFrame {
    /// Frame text.
    pub text: String,
    /// Who produced the frame. Used in the archive stub.
    pub source: String,
}

/// Projected token count for one frame. Floor of one keeps empty frames
/// visible to the budget instead of vanishing.
pub fn project_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

/// Result of one compaction pass.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CompactionOutcome {
    /// Frames retained, in original order.
    pub kept: Vec<ContextFrame>,
    /// Frames archived, oldest first.
    pub archived: Vec<ContextFrame>,
}

/// Compact `frames` to fit `budget` projected tokens. Newest frames win:
/// walk backward, keep what fits, archive the rest oldest-first.
pub fn compact_frames(frames: Vec<ContextFrame>, budget: usize) -> CompactionOutcome {
    let mut spent = 0usize;
    let mut keep_from = frames.len();
    for (index, frame) in frames.iter().enumerate().rev() {
        let cost = project_tokens(&frame.text);
        if spent + cost > budget {
            break;
        }
        spent += cost;
        keep_from = index;
    }
    let kept = frames.iter().skip(keep_from).cloned().collect::<Vec<ContextFrame>>();
    let archived = frames[..keep_from].to_vec();
    if !archived.is_empty() {
        tracing::info!(archived = archived.len(), kept = kept.len(), budget, "context compacted");
    }
    CompactionOutcome {
        kept,
        archived,
    }
}
