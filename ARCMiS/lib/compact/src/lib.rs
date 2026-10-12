//! `mas_compact` is the snapcompact-style context compaction (ADR 0031).
//!
//! One context frame is a list of byte slices. The token budget
//! projection is `bytes / 4` with a floor of one, matching the retired
//! `hook.rs` threshold arithmetic exactly. Compaction walks backward
//! from the newest frame, keeps what fits, and archives the rest in
//! oldest-first order.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod compact_frames;
pub mod compaction_outcome;
pub mod context_frame;
pub mod project_tokens;

pub use compact_frames::compact_frames;
pub use compaction_outcome::CompactionOutcome;
pub use context_frame::ContextFrame;
pub use project_tokens::project_tokens;
