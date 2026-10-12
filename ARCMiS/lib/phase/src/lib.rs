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

pub mod evidence;
pub mod phase;
pub mod phase_machine;
pub mod transition;

pub use evidence::Evidence;
pub use phase::Phase;
pub use phase_machine::PhaseMachine;
pub use transition::Transition;
