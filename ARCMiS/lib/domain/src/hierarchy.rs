//! Static team registry compiled once per run (ADR 0031).
//!
//! Tier 1 holds the orchestrator plus the advisory fleet-analyst. Tier 2
//! holds the leads. Tier 3 holds the frozen specialist roster. The
//! registry maps each specialist to its lead and answers fanout checks.
//!
//! The roster grew from ten to twenty-two variants per ADR 0038. The
//! ten old variants stay for backward compatibility; they are marked
//! `#[deprecated]` and resolve through the alias table.

pub use crate::role::Role;
pub use crate::team::Team;
pub use crate::team_registry::TeamRegistry;
