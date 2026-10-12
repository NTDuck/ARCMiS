//! Projected token count for one frame.
//!
//! Floor of one keeps empty frames visible to the budget instead of
//! vanishing.

/// Projected token count for one frame. Floor of one keeps empty frames
/// visible to the budget instead of vanishing.
pub fn project_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}
