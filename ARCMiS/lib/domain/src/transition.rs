//! One legal phase transition, with the reason it fired.

use crate::phase::Phase;

/// One legal phase transition, with the reason it fired.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    /// Phase before the transition.
    pub from: Phase,
    /// Phase after the transition.
    pub to: Phase,
    /// Why the transition fired.
    pub reason: String,
}
