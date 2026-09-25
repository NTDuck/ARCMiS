//! Workspace layout: the run's filesystem contract.
//!
//! `source/` is the read-only copy of the input codebase; `analysis/` holds
//! analyst/architect/planner artifacts; `target/` holds the migration output;
//! `runs/` holds per-run logs.

use anyhow::Context as _;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

/// The migration workspace.
#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// Bind a workspace at `root` (typically `{output_dir}/workspace`).
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
        }
    }

    /// Workspace root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Read-only source copy.
    #[must_use]
    pub fn source(&self) -> PathBuf {
        self.root.join("source")
    }

    /// Writable analysis artifacts.
    #[must_use]
    pub fn analysis(&self) -> PathBuf {
        self.root.join("analysis")
    }

    /// Writable migration output.
    #[must_use]
    pub fn target(&self) -> PathBuf {
        self.root.join("target")
    }

    /// Run logs.
    #[must_use]
    pub fn runs(&self) -> PathBuf {
        self.root.join("runs")
    }

    /// Copy the source codebase into `source/` once. Fails if `source/`
    /// already holds content: the snapshot is a run-start invariant.
    pub fn snapshot_source(&self, src: &Path) -> anyhow::Result<usize> {
        let target = self.source();
        std::fs::create_dir_all(&target)?;
        let mut copied = 0usize;
        copy_tree(src, &target, &mut copied)?;
        Ok(copied)
    }

    /// Refuse to write into `source/`. The guard calls this before any tool
    /// whose metadata says WriteLocal targets the source tree.
    pub fn assert_source_readonly(&self, path: &Path) -> anyhow::Result<()> {
        let source = self.source();
        // Tool paths resolve from the workspace root (relative paths like
        // `source/main.c` are the common case); absolutize before comparing
        // so a relative path cannot bypass the check.
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        let canonical_target = absolute.canonicalize().unwrap_or(absolute);
        let canonical_source = source.canonicalize().unwrap_or(source);
        if canonical_target.starts_with(&canonical_source) {
            anyhow::bail!(
                "'{}' is inside source/, which is read-only. Edit the copy under target/ instead.",
                path.display()
            )
        }
        Ok(())
    }
}

/// Recursively copy one directory, returning the file count.
fn copy_tree(src: &Path, dst: &Path, copied: &mut usize) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in WalkDir::new(src).min_depth(1) {
        let entry = entry.with_context(|| format!("walk {}", src.display()))?;
        let relative = entry.path().strip_prefix(src)?;
        let destination = dst.join(relative);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&destination)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &destination)?;
            *copied += 1;
        }
    }
    Ok(())
}
