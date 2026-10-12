//! One legal module transition, with the reason it fired.

use crate::module_phase::ModulePhase;

/// One legal module transition, with the reason it fired.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleTransition {
    /// Phase before the transition.
    pub from: ModulePhase,
    /// Phase after the transition.
    pub to: ModulePhase,
    /// Why the transition fired.
    pub reason: String,
}
