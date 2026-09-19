//! Sandbox path helpers shared by the search tools.

use crate::util::path::path_sanitize;
use std::path::Path;
use std::path::PathBuf;

/// Resolve requested root paths under the sandbox root. An absent or empty
/// list resolves to the root itself.
pub fn resolve_roots(root: &Path, paths: Option<&[String]>) -> Result<Vec<PathBuf>, String> {
    let requested = match paths {
        Some(paths) if !paths.is_empty() => paths,
        _ => &[".".to_owned()][..],
    };
    requested.iter().map(|path| path_sanitize(root, path)).collect()
}

/// Render one relative display path for a path under the sandbox.
pub fn relative_path(sandbox: &Path, path: &Path) -> String {
    path.strip_prefix(sandbox)
        .map(|relative| relative.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}
