//! One methodology phase.
//!
//! The run-level FSM grows from ten to fifteen states per ADR 0039.
//! The ten original variants stay for backward compatibility and are
//! marked `#[deprecated]`; the five new variants slot into the forward
//! walk between `Preflight` and `Done`.

use serde::Serialize;

/// One methodology phase.
///
/// The run-level FSM grows from ten to fifteen states per ADR 0039.
/// The ten original variants stay for backward compatibility and are
/// marked `#[deprecated]`; the five new variants slot into the forward
/// walk between `Preflight` and `Done`.
#[allow(deprecated)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Phase {
    /// Deterministic initialization before any model call.
    Init,
    /// Deterministic validation before any model call.
    #[deprecated(since = "0.2.0", note = "Preflight keeps its name; the new walk adds Init before it")]
    Preflight,
    /// Repository assessment: size, complexity, risk.
    Assess,
    /// Source exploration.
    #[deprecated(since = "0.2.0", note = "use Phase::Map")]
    Discovery,
    /// Source map published.
    Map,
    /// Business rules extraction from source code.
    ExtractRules,
    /// Citation audit of extracted rules.
    ReviewRules,
    /// Synthesis document.
    #[deprecated(since = "0.2.0", note = "use Phase::DraftBrief")]
    Contract,
    /// Migration brief published.
    DraftBrief,
    /// Dependency batches.
    #[deprecated(since = "0.2.0", note = "use Phase::MigratePilot")]
    Planning,
    /// Pilot module migrated.
    MigratePilot,
    /// One batch end-to-end to validate the plan.
    #[deprecated(since = "0.2.0", note = "use Phase::VerifyPilot")]
    Pilot,
    /// Pilot module verified.
    VerifyPilot,
    /// Remaining batches.
    #[deprecated(since = "0.2.0", note = "use Phase::MigrateBatches")]
    Migration,
    /// All modules migrated.
    MigrateBatches,
    /// Test translation and characterization.
    #[deprecated(since = "0.2.0", note = "use Phase::VerifyModule")]
    Integration,
    /// All modules verified.
    VerifyModule,
    /// Toolchain rerun as the only scorer.
    #[deprecated(since = "0.2.0", note = "use Phase::FinalVerify")]
    FinalValidation,
    /// Toolchain green.
    FinalVerify,
    /// Adversarial review.
    #[deprecated(since = "0.2.0", note = "use Phase::Harden")]
    Hardening,
    /// Security audit complete.
    Harden,
    /// Human approval gate before Done.
    HumanSignOff,
    /// Run finished.
    #[deprecated(since = "0.2.0", note = "Done keeps its name; the new walk adds HumanSignOff before it")]
    Done,
}

impl Phase {
    /// Fixed forward walk order, for progress reporting.
    #[allow(deprecated)]
    pub const ORDER: [Phase; 15] = [
        Phase::Init,
        Phase::Preflight,
        Phase::Assess,
        Phase::Map,
        Phase::ExtractRules,
        Phase::ReviewRules,
        Phase::DraftBrief,
        Phase::MigratePilot,
        Phase::VerifyPilot,
        Phase::MigrateBatches,
        Phase::VerifyModule,
        Phase::FinalVerify,
        Phase::Harden,
        Phase::HumanSignOff,
        Phase::Done,
    ];

    /// Map a deprecated variant to its forward-walk successor.
    #[allow(deprecated)]
    fn successor(self) -> Phase {
        match self {
            Phase::Discovery => Phase::Map,
            Phase::Contract => Phase::DraftBrief,
            Phase::Planning => Phase::MigratePilot,
            Phase::Pilot => Phase::VerifyPilot,
            Phase::Migration => Phase::MigrateBatches,
            Phase::Integration => Phase::VerifyModule,
            Phase::FinalValidation => Phase::FinalVerify,
            Phase::Hardening => Phase::Harden,
            other => other,
        }
    }

    /// Position in the forward order. `Done` ranks last.
    pub fn position(self) -> usize {
        let resolved = self.successor();
        Phase::ORDER.iter().position(|step| *step == resolved).unwrap_or(Phase::ORDER.len() - 1)
    }

    /// The forward successor. `Done` returns itself.
    pub fn forward_next(self) -> Phase {
        let resolved = self.successor();
        Phase::ORDER.get(resolved.position() + 1).copied().unwrap_or(resolved)
    }

    /// The regression target of `phase`, or `None` when none exists.
    /// Repeated regression burns staging work: the caller should halt.
    #[allow(deprecated)]
    pub fn regression_target(phase: Phase) -> Option<Phase> {
        use Phase::*;
        match phase {
            // New walk regression edges (ADR 0039 section 4).
            Assess => Some(Preflight),
            Map => Some(Assess),
            ExtractRules => Some(Map),
            ReviewRules => Some(ExtractRules),
            DraftBrief => Some(ReviewRules),
            MigratePilot => Some(DraftBrief),
            VerifyPilot => Some(MigratePilot),
            MigrateBatches => Some(VerifyPilot),
            VerifyModule => Some(MigrateBatches),
            FinalVerify => Some(VerifyModule),
            Harden => Some(FinalVerify),
            // Legacy regression edges for backward compatibility.
            Pilot | Migration => Some(Discovery),
            Integration | Hardening | FinalValidation => Some(Integration),
            _ => None,
        }
    }
}
