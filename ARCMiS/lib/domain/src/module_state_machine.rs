//! The per-module state machine.
//!
//! `try_advance` moves only when the edge is legal and the repair
//! budget permits. Every transition logs, so one module run is
//! traceable.

use crate::module_phase::ModulePhase;
use crate::module_transition::ModuleTransition;

/// The per-module state machine. `try_advance` moves only when the edge
/// is legal and the repair budget permits. Every transition logs, so one
/// module run is traceable.
#[derive(Debug)]
pub struct ModuleStateMachine {
    phase: ModulePhase,
    history: Vec<ModuleTransition>,
    repairs_used: usize,
    max_repairs: usize,
}

impl ModuleStateMachine {
    /// A machine starting at `Planned` with `max_repairs` repair budget.
    pub fn new(max_repairs: usize) -> Self {
        Self {
            phase: ModulePhase::Planned,
            history: Vec::new(),
            repairs_used: 0,
            max_repairs,
        }
    }

    /// Current phase.
    pub fn phase(&self) -> ModulePhase {
        self.phase
    }

    /// Number of repair cycles consumed so far.
    pub fn repairs_used(&self) -> usize {
        self.repairs_used
    }

    /// Repair budget configured at construction.
    pub fn max_repairs(&self) -> usize {
        self.max_repairs
    }

    /// Try to move to `to` under `reason`. Returns the transition or
    /// holds with the reason.
    pub fn try_advance(&mut self, to: ModulePhase, reason: &str) -> Result<ModuleTransition, String> {
        if !Self::is_legal(self.phase, to) {
            return Err(format!("illegal module transition {:?} to {:?}", self.phase, to));
        }
        if to == ModulePhase::Escalated && self.repairs_used < self.max_repairs {
            return Err(format!(
                "module escalation requires repairs_used ({}) >= max_repairs ({})",
                self.repairs_used, self.max_repairs
            ));
        }
        if to == ModulePhase::Repairing {
            self.repairs_used += 1;
        }
        let transition = ModuleTransition {
            from: self.phase,
            to,
            reason: reason.to_owned(),
        };
        tracing::info!(
            from = ?transition.from,
            to = ?transition.to,
            repairs_used = self.repairs_used,
            reason,
            "module phase advanced"
        );
        self.phase = to;
        self.history.push(transition.clone());
        Ok(transition)
    }

    /// Whether `to` is a legal successor of `from`.
    pub fn is_legal(from: ModulePhase, to: ModulePhase) -> bool {
        use ModulePhase::*;
        matches!(
            (from, to),
            // Forward walk.
            (Planned, Ready)
                | (Ready, Baselined)
                | (Baselined, Migrating)
                | (Migrating, Verifying)
                | (Verifying, Proven)
                // Failure transitions.
                | (Migrating, Diagnosing)
                | (Diagnosing, Repairing)
                | (Repairing, Migrating)
                | (Migrating, Escalated)
                | (Verifying, Diagnosing)
                | (Repairing, Verifying)
                | (Verifying, Escalated)
                // Regression edges (ADR 0039 section 4).
                | (Baselined, Ready)
                | (Migrating, Baselined)
                | (Verifying, Migrating)
                | (Diagnosing, Baselined)
                | (Repairing, Diagnosing)
        )
    }

    /// Whether the machine reached a terminal phase.
    pub fn is_terminal(&self) -> bool {
        self.phase == ModulePhase::Proven || self.phase == ModulePhase::Escalated
    }

    /// Trace of every accepted transition, in flight order.
    pub fn history(&self) -> &[ModuleTransition] {
        &self.history
    }
}
