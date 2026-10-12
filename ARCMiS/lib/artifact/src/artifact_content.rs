//! The body of one artifact.

use std::path::PathBuf;

/// The body of one artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactContent {
    /// UTF-8 text body.
    Text(String),
    /// Opaque bytes.
    Binary(Vec<u8>),
    /// Reference to a file on disk.
    Reference(PathBuf),
}
