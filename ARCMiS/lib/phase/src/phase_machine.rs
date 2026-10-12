//! The phase state machine.

use crate::evidence::Evidence;
use crate::phase::Phase;
use crate::transition::Transition;

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
