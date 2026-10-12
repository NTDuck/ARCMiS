//! `mas_phase` is the evidence-gated phase machine with deterministic
//! regression edges (ADR 0031, ADR 0039).
//!
//! Forward movement requires evidence-gated phase exit: a phase ends only
//! when its gate criteria hold. Failures regress backwards along the
//! fixed edges. `Repair` from PRODUCT-SPECS is not a phase here. Rework
//! folds into the task-level rework loop inside Migration and
//! Integration, bounded by `limits.max_repairs`.
//!
//! NOTE: this module gates phase exits. Validation names judging work
//! that the `Validator` role owns.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

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

/// Typed gate criteria one phase exit measured. Every variant names its
/// evidence precisely: no evidence means no advance.
#[derive(Debug, Clone, PartialEq)]
pub enum Evidence {
    /// The phase saw at least one completed dispatch.
    CompletedDispatch,
    /// The phase saw at least `minimum` deliverables published.
    Deliverables {
        /// Number of deliverables published.
        published: usize,
        /// Minimum deliverables required.
        minimum: usize,
    },
    /// The phase recorded a green test-command run.
    TestCommandGreen,
}

impl Evidence {
    /// Whether this evidence satisfies its own gate.
    pub fn holds(&self) -> bool {
        match self {
            Evidence::CompletedDispatch => true,
            Evidence::Deliverables {
                published,
                minimum,
            } => published >= minimum,
            Evidence::TestCommandGreen => true,
        }
    }
}

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

/// The phase state machine. `advance` moves only when evidence holds and
/// the edge is legal. Every transition logs, so one run is traceable.
#[derive(Debug)]
pub struct PhaseMachine {
    /// Current phase.
    phase: Phase,
    /// Trace of every accepted transition, in flight order.
    pub history: Vec<Transition>,
}

impl Default for PhaseMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl PhaseMachine {
    /// A machine starting at Preflight.
    #[allow(deprecated)]
    pub fn new() -> Self {
        Self::init(Phase::Preflight)
    }

    /// A machine starting at `phase`.
    pub fn init(phase: Phase) -> Self {
        Self {
            phase,
            history: Vec::new(),
        }
    }

    /// Current phase.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Try to move to `to` under `evidence`. Returns the transition or
    /// holds with the reason.
    #[allow(deprecated)]
    pub fn try_advance(&mut self, to: Phase, reason: &str, evidence: Option<Evidence>) -> Result<Transition, String> {
        if !Self::is_legal(self.phase, to) {
            return Err(format!("illegal phase transition {:?} to {:?}", self.phase, to));
        }
        let Some(gate) = evidence else {
            return Err(format!("phase {:?} gate holds no evidence", self.phase));
        };
        if !gate.holds() {
            return Err(format!("phase {:?} gate not met", self.phase));
        }
        Ok(self.accept(to, reason))
    }

    /// Move to `to` without evidence validation. Used for deterministic
    /// bookkeeping passes only.
    pub fn accept(&mut self, to: Phase, reason: &str) -> Transition {
        let transition = Transition {
            from: self.phase,
            to,
            reason: reason.to_owned(),
        };
        tracing::info!(from = ?transition.from, to = ?transition.to, reason, "phase advanced");
        self.phase = to;
        self.history.push(transition.clone());
        transition
    }

    /// Whether `to` is a legal successor of `from`. Regressions cover the
    /// phases whose work can rework the failure. Plan gaps re-enter
    /// Discovery, batch rework re-enters Migration, review and validation
    /// failures re-enter Integration.
    #[allow(deprecated)]
    pub fn is_legal(from: Phase, to: Phase) -> bool {
        use Phase::*;
        matches!(
            (from, to),
            // New forward walk (ADR 0039 section 1).
            (Init, Preflight)
                | (Preflight, Assess)
                | (Assess, Map)
                | (Map, ExtractRules)
                | (ExtractRules, ReviewRules)
                | (ReviewRules, DraftBrief)
                | (DraftBrief, MigratePilot)
                | (MigratePilot, VerifyPilot)
                | (VerifyPilot, MigrateBatches)
                | (MigrateBatches, VerifyModule)
                | (VerifyModule, FinalVerify)
                | (FinalVerify, Harden)
                | (Harden, HumanSignOff)
                | (HumanSignOff, Done)
                // New regression edges (ADR 0039 section 4).
                | (Assess, Preflight)
                | (Map, Assess)
                | (ExtractRules, Map)
                | (ReviewRules, ExtractRules)
                | (DraftBrief, ReviewRules)
                | (MigratePilot, DraftBrief)
                | (VerifyPilot, MigratePilot)
                | (MigrateBatches, VerifyPilot)
                | (VerifyModule, MigrateBatches)
                | (FinalVerify, VerifyModule)
                | (Harden, FinalVerify)
                // Legacy forward walk for backward compatibility.
                | (Preflight, Discovery)
                | (Discovery, Contract)
                | (Contract, Planning)
                | (Planning, Pilot)
                | (Pilot, Migration)
                | (Migration, Integration)
                | (Integration, Hardening)
                | (Hardening, FinalValidation)
                | (FinalValidation, Done)
                // Legacy regression edges for backward compatibility.
                | (Pilot, Discovery)
                | (Migration, Discovery)
                | (Integration, Migration)
                | (Hardening, Integration)
                | (FinalValidation, Integration)
        )
    }
}
