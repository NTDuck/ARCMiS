//! Migration tools for ARCMiS. Each tool wraps one filesystem or process
//! operation the agent may request.

use ::serde::Deserialize;
use ::serde_json::json;

/// Read a file from the workspace.
pub struct ReadFile {
    /// Root directory; tool paths resolve inside it.
    pub root: ::std::path::PathBuf,
}

/// Normalize a tool-supplied path under `root`. Rejects absolute paths and
/// `..` components so writes cannot escape the sandbox. Returns the joined
/// path or an error message for the model.
fn sanitize(root: &::std::path::Path, path: &str) -> ::core::result::Result<::std::path::PathBuf, String> {
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
    Ok(root.join(rel))
}

/// Paths the agent may never write, supplied by the caller from config.
#[derive(Default)]
pub struct ProtectedFiles(pub ::std::vec::Vec<::std::string::String>);

impl ProtectedFiles {
    fn blocks(&self, file_name: &str) -> bool {
        self.0.iter().any(|f| f == file_name)
    }
}

#[derive(::core::fmt::Debug, Deserialize)]
pub struct ReadFileArgs {
    pub path: ::std::string::String,
}

impl ::rig::tool::Tool for ReadFile {
    const NAME: &'static str = "read_file";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = ReadFileArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> String {
        "Read a file from the workspace. Args: {\"path\": \"relative/path\"}".to_owned()
    }

    fn parameters(&self) -> ::serde_json::Value {
        json!({
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
        let path = sanitize(&self.root, &args.path).map_err(::rig::tool::ToolExecutionError::other)?;
        let content = ::std::fs::read_to_string(&path)
            .map_err(|e| ::rig::tool::ToolExecutionError::other(format!("read_file failed for {}: {e}", args.path)))?;
        Ok(::rig::tool::ToolOutput::text(content))
    }
}

/// Write a file inside the output workspace.
pub struct WriteFile {
    /// Root directory; tool paths resolve inside it.
    pub root: ::std::path::PathBuf,
    /// File names the agent may not write.
    pub protected: ProtectedFiles,
}

#[derive(::core::fmt::Debug, Deserialize)]
pub struct WriteFileArgs {
    pub path: ::std::string::String,
    pub content: ::std::string::String,
}

impl ::rig::tool::Tool for WriteFile {
    const NAME: &'static str = "write_file";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = WriteFileArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> String {
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
        let path = sanitize(&self.root, &args.path).map_err(::rig::tool::ToolExecutionError::other)?;
        // The package manifest is toolchain-owned (the harness writes the
        // scaffold). Fail closed with actionable feedback for the model.
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if self.protected.blocks(file_name) {
            return ::core::result::Result::Err(::rig::tool::ToolExecutionError::other(format!(
                "'{file_name}' is owned by the toolchain and already exists in the output workspace. Do not write it. Continue with the source and test files."
            )));
        }
        if let Some(parent) = path.parent() {
            ::std::fs::create_dir_all(parent).map_err(|e| {
                ::rig::tool::ToolExecutionError::other(format!("write_file failed for {}: {e}", args.path))
            })?;
        }
        let len = args.content.len();
        ::std::fs::write(&path, args.content)
            .map_err(|e| ::rig::tool::ToolExecutionError::other(format!("write_file failed for {}: {e}", args.path)))?;
        Ok(::rig::tool::ToolOutput::text(format!("wrote {} ({len} bytes)", args.path)))
    }
}

/// Run a command in a working directory.
pub struct RunCommand {
    /// Working directory for the command.
    pub cwd: ::std::path::PathBuf,
}

#[derive(::core::fmt::Debug, Deserialize)]
pub struct RunCommandArgs {
    pub program: ::std::string::String,
    #[serde(default)]
    pub args: ::std::vec::Vec<::std::string::String>,
}

/// Captured result of one command run.
#[derive(::core::fmt::Debug)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: ::std::string::String,
    pub stderr: ::std::string::String,
}

impl ::rig::tool::Tool for RunCommand {
    const NAME: &'static str = "run_command";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = RunCommandArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> String {
        "Run a command in the workspace. Args: {\"program\": \"cargo\", \"args\": [\"test\"]}".to_owned()
    }

    fn parameters(&self) -> ::serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "program": { "type": "string", "description": "Executable name" },
                "args": { "type": "array", "items": { "type": "string" }, "description": "Argument list" }
            },
            "required": ["program"]
        })
    }

    async fn call(
        &self,
        _context: &mut ::rig::tool::ToolContext,
        args: Self::Args,
    ) -> ::core::result::Result<Self::Output, Self::Error> {
        let output = ::tokio::process::Command::new(&args.program)
            .args(&args.args)
            .current_dir(&self.cwd)
            .output()
            .await
            .map_err(|e| {
                ::rig::tool::ToolExecutionError::other(format!("run_command failed for {}: {e}", args.program))
            })?;
        let result = CommandOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        };
        Ok(::rig::tool::ToolOutput::json(::serde_json::json!({
            "exit_code": result.exit_code,
            "stdout": result.stdout,
            "stderr": result.stderr,
        })))
    }
}
