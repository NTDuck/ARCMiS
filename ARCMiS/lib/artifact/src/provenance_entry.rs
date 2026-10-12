//! One entry in the provenance chain.

use std::time::SystemTime;

use crate::provenance_action::ProvenanceAction;

/// One entry in the provenance chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceEntry {
    /// Agent that performed the action.
    pub agent: String,
    /// Action performed.
    pub action: ProvenanceAction,
    /// When the action happened.
    pub timestamp: SystemTime,
    /// Content hash at the time of the action.
    pub content_hash: String,
}
