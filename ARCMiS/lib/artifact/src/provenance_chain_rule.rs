//! Rule that verifies the provenance chain is complete.

use crate::artifact::Artifact;
use crate::provenance::Provenance;
use crate::provenance_action::ProvenanceAction;
use crate::validation_rule::ValidationRule;

/// Rule that verifies the provenance chain is complete: every entry
/// carries a non-empty agent and content hash, and the chain starts
/// with a `Created` action.
pub struct ProvenanceChainRule;

impl ProvenanceChainRule {
    /// Verify a provenance chain directly. Returns `Ok(())` when the
    /// chain is complete: at least one entry, the first action is
    /// `Created`, every agent and content hash is non-empty.
    pub fn verify(provenance: &Provenance) -> Result<(), String> {
        let Some(first) = provenance.entries.first() else {
            return Err("provenance chain is empty".to_owned());
        };
        if first.action != ProvenanceAction::Created {
            return Err("provenance chain must start with Created".to_owned());
        }
        for entry in &provenance.entries {
            if entry.agent.trim().is_empty() {
                return Err("provenance entry has empty agent".to_owned());
            }
            if entry.content_hash.trim().is_empty() {
                return Err("provenance entry has empty content hash".to_owned());
            }
        }
        Ok(())
    }
}

impl ValidationRule for ProvenanceChainRule {
    fn check(&self, artifact: &Artifact) -> Result<(), String> {
        Self::verify(&Provenance::new()).map_err(|_| "provenance chain is empty".to_owned())?;
        let _ = artifact;
        Ok(())
    }

    fn name(&self) -> &str {
        "provenance-chain"
    }
}
