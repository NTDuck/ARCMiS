//! `eval` runs short code cells in Python or JavaScript.
//!
//! Each cell runs in a fresh `python3 -c` or `bun -e` process. A later cell
//! does not see the state of an earlier cell. Persistent per-language kernels
//! are a known deferral and land in a later pass. Each cell has its own
//! timeout in seconds with default 300, clamped to the range 1 through 3600.
//! A failed cell stops the run. Remaining cells report the status `skipped`.

/// `eval` runs code cells in Python or JavaScript and reports each result.
pub struct Eval {}

impl rig::tool::Tool for Eval {
    const NAME: &'static str = "eval";
    type Error = rig::tool::ToolExecutionError;
    type Args = EvalArgs;
    type Output = rig::tool::ToolOutput;

    fn description(&self) -> std::string::String {
        "Run short code cells in Python or JavaScript and report each result.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "cells": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "language": {
                                "type": "string",
                                "enum": ["py", "js"],
                                "description": "Cell language, py or js"
                            },
                            "code": {
                                "type": "string",
                                "description": "Code text for this cell"
                            },
                            "title": {
                                "type": "string",
                                "description": "Short label for this cell"
                            },
                            "timeout": {
                                "type": "number",
                                "description": "Cell timeout in seconds, default 300, range 1 to 3600"
                            },
                            "reset": {
                                "type": "boolean",
                                "description": "Ignored now. Kernels are not persistent yet"
                            }
                        },
                        "required": ["language", "code"]
                    },
                    "description": "Code cells in run order"
                }
            },
            "required": ["cells"]
        })
    }

    async fn call(
        &self,
        _context: &mut rig::tool::ToolContext,
        args: Self::Args,
    ) -> core::result::Result<Self::Output, Self::Error> {
        if args.cells.is_empty() {
            return core::result::Result::Err(rig::tool::ToolExecutionError::invalid_args(
                "cells must hold at least one cell.".to_owned(),
            ));
        }
        let total = args.cells.len();
        let mut failed = false;
        let mut report = std::string::String::new();
        for (index, cell) in args.cells.into_iter().enumerate() {
            let number = index + 1;
            let label = cell.title.clone().unwrap_or_else(|| cell.language.clone());
            if failed {
                report.push_str(&std::format!("[{number}/{total}] {label}: skipped\n"));
                continue;
            }
            let seconds = clamp_seconds(cell.timeout.unwrap_or(DEFAULT_TIMEOUT));
            let run = run_cell(&cell.language, &cell.code, seconds).await;
            let run = match run {
                core::result::Result::Ok(run) => run,
                core::result::Result::Err(error) => {
                    report.push_str(&std::format!("[{number}/{total}] {label}: failed\n{error}\n"));
                    failed = true;
                    continue;
                },
            };
            let output = truncate_output(&run.output);
            report.push_str(&std::format!("[{number}/{total}] {label}\n{output}\n"));
            if run.code != 0 {
                report.push_str(&std::format!("[{number}/{total}] {label}: exited with code {}\n", run.code));
                failed = true;
            }
        }
        core::result::Result::Ok(rig::tool::ToolOutput::text(report))
    }
}

/// Arguments for `eval`.
#[derive(Debug, serde::Deserialize)]
pub struct EvalArgs {
    pub cells: std::vec::Vec<EvalCell>,
}

/// One code cell of the eval run.
#[derive(Debug, serde::Deserialize)]
pub struct EvalCell {
    pub language: std::string::String,
    pub code: std::string::String,
    pub title: core::option::Option<std::string::String>,
    pub timeout: core::option::Option<u64>,
    /// Requested state reset. No persistent kernel exists yet.
    pub reset: core::option::Option<bool>,
}

/// Default cell timeout in seconds.
const DEFAULT_TIMEOUT: u64 = 300;

/// Output size kept before truncation.
const OUTPUT_LIMIT: usize = 50 * 1024;

/// Clamp a timeout to the range 1 through 3600 seconds.
fn clamp_seconds(timeout: u64) -> u64 {
    timeout.clamp(1, 3600)
}

/// Run one cell in a fresh interpreter process and capture its output.
async fn run_cell(language: &str, code: &str, seconds: u64) -> core::result::Result<Captured, std::string::String> {
    match language {
        "py" => {
            let arguments = std::vec!["-c".to_owned(), code.to_owned()];
            capture("python3", &arguments, seconds).await
        },
        "js" => {
            let arguments = std::vec!["-e".to_owned(), code.to_owned()];
            capture("bun", &arguments, seconds).await
        },
        other => core::result::Result::Err(std::format!("language '{other}' is not supported. Use py or js.")),
    }
}

/// Spawn one program with a watchdog timeout and capture its output.
async fn capture(
    program: &str,
    arguments: &[std::string::String],
    seconds: u64,
) -> core::result::Result<Captured, std::string::String> {
    let mut command = tokio::process::Command::new(program);
    command.args(arguments).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    let mut child = command.spawn().map_err(|error| std::format!("{program} failed to start: {error}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let capture = tokio::time::timeout(
        std::time::Duration::from_secs(seconds),
        std::pin::pin!(async {
            let (out_bytes, err_bytes, status) = tokio::join!(drain(stdout), drain(stderr), child.wait());
            (out_bytes, err_bytes, status)
        }),
    )
    .await;
    let (stdout_bytes, stderr_bytes, status) = match capture {
        core::result::Result::Ok(parts) => parts,
        core::result::Result::Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return core::result::Result::Err(std::format!("{program} timed out after {seconds} seconds"));
        },
    };
    let code = match status {
        core::result::Result::Ok(status) => status.code().unwrap_or(-1),
        core::result::Result::Err(error) => {
            return core::result::Result::Err(std::format!("{program} status failed: {error}"));
        },
    };
    let mut output = std::string::String::from_utf8_lossy(&stdout_bytes).into_owned();
    output.push_str(&std::string::String::from_utf8_lossy(&stderr_bytes));
    core::result::Result::Ok(Captured {
        code,
        output,
    })
}

/// Read one pipe to the end and return its bytes.
async fn drain<R>(pipe: core::option::Option<R>) -> std::vec::Vec<u8>
where
    R: tokio::io::AsyncRead + core::marker::Unpin,
{
    let mut pipe = match pipe {
        Some(pipe) => pipe,
        None => return std::vec::Vec::new(),
    };
    let mut bytes = std::vec::Vec::new();
    let _ = tokio::io::AsyncReadExt::read_to_end(&mut pipe, &mut bytes).await;
    bytes
}

/// Cut output at 50 KiB and mark the cut.
fn truncate_output(output: &str) -> std::string::String {
    if output.len() <= OUTPUT_LIMIT {
        return output.to_owned();
    }
    let cut = std::string::String::from_utf8_lossy(&output.as_bytes()[..OUTPUT_LIMIT]).into_owned();
    std::format!("{cut}\n[output truncated at 50 KiB]")
}

/// Captured result of one process run.
struct Captured {
    code: i32,
    output: std::string::String,
}
