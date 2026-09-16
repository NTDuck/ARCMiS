//! `read_file` reads one file from the workspace.


/// Arguments for `read_file`.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
pub struct ReadFileArgs {
    pub path: ::std::string::String,
}

/// Resolve the tool path under the workspace root.
fn resolve(
    root: &::std::path::Path,
    path: &str,
) -> ::core::result::Result<::std::path::PathBuf, ::std::string::String> {
    crate::path::path_sanitize(root, path)
}

/// Read the file content as UTF-8 text.
fn read(
    path: &::std::path::Path,
    requested: &str,
) -> ::core::result::Result<::std::string::String, ::rig::tool::ToolExecutionError> {
    ::std::fs::read_to_string(path)
        .map_err(|error| ::rig::tool::ToolExecutionError::other(::std::format!("read_file failed for {requested}: {error}")))
}

/// `read_file` returns the content of one file in the workspace.
pub struct ReadFile {
    /// Root directory. Tool paths resolve inside it.
    pub root: ::std::path::PathBuf,
}

impl ::rig::tool::Tool for ReadFile {
    const NAME: &'static str = "read_file";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = ReadFileArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> ::std::string::String {
        "Read one file from the workspace and return its content.".to_owned()
    }

    fn parameters(&self) -> ::serde_json::Value {
        ::serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path relative to the workspace root" }
            },
            "required": ["path"]
        })
    }

    async fn call(
        &self,
        _context: &mut ::rig::tool::ToolContext,
        args: Self::Args,
    ) -> ::core::result::Result<Self::Output, Self::Error> {
        let path = resolve(&self.root, &args.path).map_err(::rig::tool::ToolExecutionError::other)?;
        let content = read(&path, &args.path)?;
        ::core::result::Result::Ok(::rig::tool::ToolOutput::text(content))
    }
}
