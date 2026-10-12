//! `mas_core` is the conductor runtime over the omp transport (ADR 0031).
//!
//! The conductor crate wires the composition crates together. It owns
//! the run lifetime, the transport handle, the task graph, the phase
//! machines, the router, the gateway, and the context store.
//!
//! Modules:
//! - `conductor` — one run: phases, dispatches, event loop.
//! - `context` — rolling context window with budget-driven compaction.
//! - `frame_sink` — maps decoded omp frames onto task-graph state changes.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod absorb_prompt_result;
pub mod absorb_response;
pub mod conductor;
pub mod context;
pub mod frame_sink;
pub mod round_outcome;
pub mod run_limits;
pub mod run_summary;

pub use conductor::Conductor;
pub use round_outcome::RoundOutcome;
pub use run_limits::RunLimits;
pub use run_summary::RunSummary;
pub use context::ContextStore;
