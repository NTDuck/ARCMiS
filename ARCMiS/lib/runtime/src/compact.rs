//! Snapcompact-style context compaction for the MAS conductor.
//!
//! One context frame is a list of byte slices. The token budget
//! projection is `bytes / 4` with a floor of one, matching the retired
//! `hook.rs` threshold arithmetic exactly. Compaction walks backward
//! from the newest frame, keeps what fits, and archives the rest in
//! oldest-first order.

pub use crate::compact_frames::compact_frames;
pub use crate::compaction_outcome::CompactionOutcome;
pub use crate::context_frame::ContextFrame;
pub use crate::project_tokens::project_tokens;
