//! `mas_gateway` is the deterministic tool-call policy (ADR 0031).
//!
//! A tool call becomes an allow or deny verdict before it reaches the
//! transport. Deny patterns use the legacy `"<tool>:<substring>"` form
//! carried from the retired `guard.rs`. When both lists match, deny
//! wins: this follows the run-config GuardConfig contract and ADR 0031
//! over any task brief asking otherwise.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

pub mod gateway;
pub mod tool_call;
pub mod tool_verdict;

pub use gateway::Gateway;
pub use tool_call::ToolCall;
pub use tool_verdict::ToolVerdict;
