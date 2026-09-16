//! `run_command` runs one command in a working directory and captures its
//! output.


/// Arguments for `run_command`.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
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

/// Start the program in the working directory and wait for it to exit.
async fn run(
    program: &str,
    args: &[::std::string::String],
    cwd: &::std::path::Path,
) -> ::core::result::Result<::std::process::Output, ::rig::tool::ToolExecutionError> {
    ::tokio::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|error| ::rig::tool::ToolExecutionError::other(::std::format!("run_command failed for {program}: {error}")))
}

/// Decode the process output into exit code, stdout, and stderr text.
fn decode(output: ::std::process::Output) -> CommandOutput {
    CommandOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: ::std::string::String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: ::std::string::String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// `run_command` runs one command in the workspace and returns its exit code
/// and captured output as JSON.
pub struct RunCommand {
    /// Working directory for the command.
    pub cwd: ::std::path::PathBuf,
}

impl ::rig::tool::Tool for RunCommand {
    const NAME: &'static str = "run_command";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = RunCommandArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> ::std::string::String {
        "Run one command in the workspace and return its exit code and output.".to_owned()
    }

    fn parameters(&self) -> ::serde_json::Value {
        ::serde_json::json!({
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
        let output = run(&args.program, &args.args, &self.cwd).await?;
        let result = decode(output);
        ::core::result::Result::Ok(::rig::tool::ToolOutput::json(::serde_json::json!({
            "exit_code": result.exit_code,
            "stdout": result.stdout,
            "stderr": result.stderr,
        })))
    }
}
