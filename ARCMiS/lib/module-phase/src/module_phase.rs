//! One per-module phase.

/// One per-module phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModulePhase {
    /// Module scheduled.
    Planned,
    /// Baseline tests captured.
    Ready,
    /// Strategy selected.
    Baselined,
    /// Translation in progress.
    Migrating,
    /// Verification in progress.
    Verifying,
    /// Tests green, artifacts complete.
    Proven,
    /// Failure classification in progress.
    Diagnosing,
    /// Fix application in progress.
    Repairing,
    /// Repair budget exhausted; human intervention required.
    Escalated,
}

impl ModulePhase {
    /// Fixed forward walk order, for progress reporting.
    pub const ORDER: [ModulePhase; 6] = [
        ModulePhase::Planned,
        ModulePhase::Ready,
        ModulePhase::Baselined,
        ModulePhase::Migrating,
        ModulePhase::Verifying,
        ModulePhase::Proven,
    ];

    /// Position in the forward order. `Proven` ranks last.
    pub fn position(self) -> usize {
        ModulePhase::ORDER.iter().position(|step| *step == self).unwrap_or(ModulePhase::ORDER.len() - 1)
    }

    /// The forward successor. `Proven` returns itself.
    pub fn forward_next(self) -> ModulePhase {
        ModulePhase::ORDER.get(self.position() + 1).copied().unwrap_or(self)
    }
}
