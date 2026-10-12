//! Compact frames to fit a token budget.

use crate::compaction_outcome::CompactionOutcome;
use crate::context_frame::ContextFrame;
use crate::project_tokens::project_tokens;

/// Compact `frames` to fit `budget` projected tokens. Newest frames win:
/// walk backward, keep what fits, archive the rest oldest-first.
pub fn compact_frames(frames: Vec<ContextFrame>, budget: usize) -> CompactionOutcome {
    let keep_from = keep_from_index(&frames, budget);
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

/// Index of the first frame to keep. Walks backward from the newest
/// frame, accumulating projected tokens until the budget is full.
fn keep_from_index(frames: &[ContextFrame], budget: usize) -> usize {
    let (keep_from, _) = frames
        .iter()
        .enumerate()
        .rev()
        .try_fold((frames.len(), 0usize), |(start, spent), (index, frame)| {
            let cost = project_tokens(&frame.text);
            if spent + cost > budget {
                Err((start, spent))
            } else {
                Ok((index, spent + cost))
            }
        })
        .unwrap_or_else(|state| state);
    keep_from
}
