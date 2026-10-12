//! Rule that verifies the artifact's content hash matches the latest
//! provenance entry.

use crate::artifact::Artifact;
use crate::artifact_content::ArtifactContent;
use crate::provenance::Provenance;
use crate::validation_rule::ValidationRule;

/// Rule that verifies the artifact's content hash matches the latest
/// provenance entry. The artifact must carry at least one provenance
/// entry whose `content_hash` equals the hash computed from the
/// artifact's content.
pub struct ContentHashRule;

impl ContentHashRule {
    /// Compute the content hash for one artifact body.
    pub fn hash(content: &ArtifactContent) -> String {
        match content {
            ArtifactContent::Text(body) => format!("text:{}:{}", body.len(), body),
            ArtifactContent::Binary(bytes) => format!("binary:{}:{}", bytes.len(), hex_preview(bytes)),
            ArtifactContent::Reference(path) => format!("ref:{}", path.display()),
        }
    }

    /// Verify that `provenance` carries a hash matching the artifact
    /// content. Returns `Ok(())` when the latest entry's hash equals
    /// the computed hash.
    pub fn verify(artifact: &Artifact, provenance: &Provenance) -> Result<(), String> {
        let expected = Self::hash(&artifact.content);
        let Some(last) = provenance.entries.last() else {
            return Err("provenance chain is empty".to_owned());
        };
        if last.content_hash != expected {
            return Err(format!("content hash mismatch: expected {expected}, got {}", last.content_hash));
        }
        Ok(())
    }
}

impl ValidationRule for ContentHashRule {
    fn check(&self, artifact: &Artifact) -> Result<(), String> {
        Self::verify(artifact, &Provenance::new())
    }

    fn name(&self) -> &str {
        "content-hash"
    }
}

fn hex_preview(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().min(16) * 2);
    for byte in bytes.iter().take(16) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
