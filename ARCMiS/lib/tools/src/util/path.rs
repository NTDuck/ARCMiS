//! Path helpers shared by tools.

/// Normalize a tool-supplied path under `root`. Reject absolute paths and
/// `..` components so writes cannot escape the sandbox. Return the joined
/// path or an error message for the model.
pub fn path_sanitize(
    root: &::std::path::Path,
    path: &str,
) -> ::core::result::Result<::std::path::PathBuf, ::std::string::String> {
    let rel = ::std::path::Path::new(path);
    if rel.is_absolute() || path.starts_with('~') {
        return ::core::result::Result::Err(format!(
            "path '{path}' is absolute. Use a path relative to the output workspace root, for example 'src/lib.rs'."
        ));
    }
    if rel.components().any(|c| c == ::std::path::Component::ParentDir) {
        return ::core::result::Result::Err(format!(
            "path '{path}' must not contain '..'. Stay inside the output workspace."
        ));
    }
    ::core::result::Result::Ok(root.join(rel))
}
