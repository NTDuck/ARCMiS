//! Path helpers shared by tools.

use std::path::Component;

/// Normalize a tool-supplied path under `root`. Reject absolute paths and
/// `..` components so writes cannot escape the sandbox. Return the joined
/// path or an error message for the model.
use std::path::Path;
use std::path::PathBuf;
pub fn path_sanitize(root: &Path, path: &str) -> Result<PathBuf, String> {
    let rel = Path::new(path);
    if rel.is_absolute() || path.starts_with('~') {
        return Err(format!(
            "path '{path}' is absolute. Use a path relative to the output workspace root, for example 'src/lib.rs'."
        ));
    }
    if rel.components().any(|c| c == Component::ParentDir) {
        return Err(format!("path '{path}' must not contain '..'. Stay inside the output workspace."));
    }
    Ok(root.join(rel))
}
