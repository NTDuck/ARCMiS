//! `mas_repository` holds repository metadata for one run.
//!
//! The metadata describes the input codebase: its root path, the
//! languages it contains, and the file count. The conductor reads the
//! metadata once at startup and never mutates it.

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

use std::path::PathBuf;

/// Repository metadata for one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryMetadata {
    /// Root path of the repository.
    pub root: PathBuf,
    /// Languages detected in the repository.
    pub languages: Vec<String>,
    /// Total file count under the root.
    pub file_count: usize,
}

impl RepositoryMetadata {
    /// Build one metadata record.
    pub fn new(root: PathBuf, languages: Vec<String>, file_count: usize) -> Self {
        Self {
            root,
            languages,
            file_count,
        }
    }
}
