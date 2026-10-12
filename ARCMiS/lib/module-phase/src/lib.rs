//! `mas_module_phase` is the per-module FSM (ADR 0039 section 2).
//!
//! The run-level FSM owns one `ModuleStateMachine` per module. A module
//! FSM advances only when the run-level FSM permits. The forward walk
//! covers `Planned` through `Proven`; failure transitions route through
//! `Diagnosing` and `Repairing`, with `Escalated` as the terminal state
//! when the repair budget is exhausted.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod module_phase;
pub mod module_state_machine;
pub mod module_transition;

pub use module_phase::ModulePhase;
pub use module_state_machine::ModuleStateMachine;
pub use module_transition::ModuleTransition;
