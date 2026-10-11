//! `phase` is the evidence-gated phase machine with deterministic
//! regression edges (ADR 0031).
//!
//! Forward movement requires evidence-gated phase exit: a phase ends only
//! when its gate criteria hold. Failures regress backwards along the
//! fixed edges. `Repair` from PRODUCT-SPECS is not a phase here. Rework
//! folds into the task-level rework loop inside Migration and
//! Integration, bounded by `limits.max_repairs`.
//!
//! NOTE: this module gates phase exits. Validation names judging work
//! that the `Validator` role owns.

use serde::Serialize;

/// One methodology phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Phase {
    /// Deterministic validation before any model call.
    Preflight,
    /// Source exploration.
    Discovery,
    /// Synthesis document.
    Contract,
    /// Dependency batches.
    Planning,
    /// One batch end-to-end to validate the plan.
    Pilot,
    /// Remaining batches.
    Migration,
    /// Test translation and characterization.
    Integration,
    /// Adversarial review.
    Hardening,
    /// Toolchain rerun as the only scorer.
    FinalValidation,
    /// Run finished.
    Done,
}

impl Phase {
    /// Fixed forward walk order, for progress reporting.
    pub const ORDER: [Phase; 10] = [
        Phase::Preflight,
        Phase::Discovery,
        Phase::Contract,
        Phase::Planning,
        Phase::Pilot,
        Phase::Migration,
        Phase::Integration,
        Phase::Hardening,
        Phase::FinalValidation,
        Phase::Done,
    ];

    /// Position in the forward order. `Done` ranks last.
    pub fn position(self) -> usize {
        Phase::ORDER.iter().position(|step| *step == self).unwrap_or(Phase::ORDER.len() - 1)
    }

    /// The forward successor. `Done` returns itself.
    pub fn forward_next(self) -> Phase {
        Phase::ORDER.get(self.position() + 1).copied().unwrap_or(self)
    }

    /// The regression target of `phase`, or `None` when none exists.
    /// Repeated regression burns staging work: the caller should halt.
    pub fn regression_target(phase: Phase) -> Option<Phase> {
        use Phase::*;
        match phase {
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
        published: usize,
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
    pub fn is_legal(from: Phase, to: Phase) -> bool {
        use Phase::*;
        matches!(
            (from, to),
            (Preflight, Discovery)
                | (Discovery, Contract)
                | (Contract, Planning)
                | (Planning, Pilot)
                | (Pilot, Migration)
                | (Migration, Integration)
                | (Integration, Hardening)
                | (Hardening, FinalValidation)
                | (FinalValidation, Done)
                | (Pilot, Discovery)
                | (Migration, Discovery)
                | (Integration, Migration)
                | (Hardening, Integration)
                | (FinalValidation, Integration)
        )
    }
}
