//! Append-only provenance chain.

use crate::provenance_entry::ProvenanceEntry;

/// Append-only provenance chain.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Provenance {
    /// Entries in append order.
    pub entries: Vec<ProvenanceEntry>,
}

impl Provenance {
    /// Empty chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one entry.
    pub fn append(&mut self, entry: ProvenanceEntry) {
        self.entries.push(entry);
    }
}
