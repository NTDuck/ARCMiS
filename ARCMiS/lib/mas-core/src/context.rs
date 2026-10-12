//! `context` is the run's rolling context window with budget-driven
//! compaction.
//!
//! Frames append in publish order. When the projected token usage
//! passes the budget, compaction keeps the newest half of the budget
//! and archives the rest (ADR 0031).

use compact::compact_frames;
use compact::project_tokens;
use compact::CompactionOutcome;
use compact::ContextFrame;

/// One run's context frames plus the compaction trigger.
#[derive(Debug)]
pub struct ContextStore {
    frames: Vec<ContextFrame>,
    budget: usize,
}

impl ContextStore {
    /// A store that compacts when usage passes `budget`.
    pub fn new(budget: usize) -> Self {
        Self {
            frames: Vec::new(),
            budget,
        }
    }

    /// Append one frame.
    pub fn publish(&mut self, source: &str, text: &str) {
        self.frames.push(ContextFrame {
            source: source.to_owned(),
            text: text.to_owned(),
        });
    }

    /// Projected token usage across all frames.
    pub fn used(&self) -> usize {
        self.frames.iter().map(|frame| project_tokens(&frame.text)).sum()
    }

    /// Compact when usage passes the budget. Newest frames win.
    pub fn settle(&mut self) {
        if self.used() > self.budget {
            let frames = std::mem::take(&mut self.frames);
            let CompactionOutcome {
                kept,
                archived,
            } = compact_frames(frames, self.budget / 2);
            self.frames = kept;
            tracing::info!(archived = archived.len(), "round compacted context");
        }
    }
}
