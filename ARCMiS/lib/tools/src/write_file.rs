//! `write_file` writes one file inside the output workspace.

use ::serde_json::json;

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

/// Arguments for `write_file`.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
pub struct WriteFileArgs {
    pub path: ::std::string::String,
    pub content: ::std::string::String,
}

/// Create the parent directories of the output path.
fn create_parents(
    path: &::std::path::Path,
    requested: &str,
) -> ::core::result::Result<(), ::rig::tool::ToolExecutionError> {
    if let ::core::option::Option::Some(parent) = path.parent() {
        ::std::fs::create_dir_all(parent).map_err(|error| {
            ::rig::tool::ToolExecutionError::other(format!("write_file failed for {requested}: {error}"))
        })?;
    }
    ::core::result::Result::Ok(())
}

/// Write the content to the output path and return the byte count.
fn write(
    path: &::std::path::Path,
    content: ::std::string::String,
    requested: &str,
) -> ::core::result::Result<Written, ::rig::tool::ToolExecutionError> {
    // Short-circuit on an identical rewrite. A model that repeats the same
    // write burns turns and wall clock. A missing file means no prior
    // content. Every other read error is real and surfaces.
    match ::std::fs::read_to_string(path) {
        ::core::result::Result::Ok(existing) if existing == content => {
            return ::core::result::Result::Ok(Written {
                bytes: content.len(),
                unchanged: true,
            });
        },
        ::core::result::Result::Err(error) if error.kind() != ::std::io::ErrorKind::NotFound => {
            return ::core::result::Result::Err(::rig::tool::ToolExecutionError::other(format!(
                "write_file failed to read {requested}: {error}"
            )));
        },
        _ => {},
    }
    let bytes = content.len();
    ::std::fs::write(path, content).map_err(|error| {
        ::rig::tool::ToolExecutionError::other(format!("write_file failed for {requested}: {error}"))
    })?;
    ::core::result::Result::Ok(Written {
        bytes,
        unchanged: false,
    })
}

/// Outcome of one `write` step.
struct Written {
    bytes: usize,
    unchanged: bool,
}

/// `write_file` writes one file inside the output workspace and creates its
/// parent directories.
pub struct WriteFile {
    /// Root directory. Tool paths resolve inside it.
    pub root: ::std::path::PathBuf,
}

impl ::rig::tool::Tool for WriteFile {
    const NAME: &'static str = "write_file";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = WriteFileArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> ::std::string::String {
        "Write a file inside the output workspace. Args: {\"path\": \"relative/path\", \"content\": \"text\"}"
            .to_owned()
    }

    fn parameters(&self) -> ::serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path relative to the output root" },
                "content": { "type": "string", "description": "Full file content" }
            },
            "required": ["path", "content"]
        })
    }

    async fn call(
        &self,
        _context: &mut ::rig::tool::ToolContext,
        args: Self::Args,
    ) -> ::core::result::Result<Self::Output, Self::Error> {
        let path = path_sanitize(&self.root, &args.path).map_err(::rig::tool::ToolExecutionError::other)?;
        create_parents(&path, &args.path)?;
        let written = write(&path, args.content, &args.path)?;
        if written.unchanged {
            ::tracing::info!(file = %args.path, "write_file skipped: content unchanged");
            return ::core::result::Result::Ok(::rig::tool::ToolOutput::text(format!(
                "write_file: skipped, content unchanged ({} bytes)",
                written.bytes
            )));
        }
        ::core::result::Result::Ok(::rig::tool::ToolOutput::text(format!(
            "wrote {} ({} bytes)",
            args.path, written.bytes
        )))
    }
}
