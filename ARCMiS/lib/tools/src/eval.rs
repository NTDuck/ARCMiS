//! `eval` runs short code cells in Python or JavaScript.
//!
//! Each cell runs in a fresh `python3 -c` or `bun -e` process. A later cell
//! does not see the state of an earlier cell. Persistent per-language kernels
//! are a known deferral and land in a later pass. Each cell has its own
//! timeout in seconds with default 300, clamped to the range 1 through 3600.
//! A failed cell stops the run. Remaining cells report the status `skipped`.

use std::pin::pin;
use std::process::Stdio;
use std::time::Duration;

/// `eval` runs code cells in Python or JavaScript and reports each result.
pub struct Eval {}

impl rig::tool::Tool for Eval {
    const NAME: &'static str = "eval";
    type Error = rig::tool::ToolExecutionError;
    type Args = EvalArgs;
    type Output = rig::tool::ToolOutput;

    fn description(&self) -> String {
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

    async fn call(&self, _context: &mut rig::tool::ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.cells.is_empty() {
            return Err(rig::tool::ToolExecutionError::invalid_args("cells must hold at least one cell.".to_owned()));
        }
        let total = args.cells.len();
        let mut failed = false;
        let mut report = String::new();
        for (index, cell) in args.cells.into_iter().enumerate() {
            let number = index + 1;
            let label = cell.title.clone().unwrap_or_else(|| cell.language.clone());
            if failed {
                report.push_str(&format!("[{number}/{total}] {label}: skipped\n"));
                continue;
            }
            let seconds = clamp_seconds(cell.timeout.unwrap_or(DEFAULT_TIMEOUT));
            let run = run_cell(&cell.language, &cell.code, seconds).await;
            let run = match run {
                Ok(run) => run,
                Err(error) => {
                    report.push_str(&format!("[{number}/{total}] {label}: failed\n{error}\n"));
                    failed = true;
                    continue;
                },
            };
            let output = truncate_output(&run.output);
            report.push_str(&format!("[{number}/{total}] {label}\n{output}\n"));
            if run.code != 0 {
                report.push_str(&format!("[{number}/{total}] {label}: exited with code {}\n", run.code));
                failed = true;
            }
        }
        Ok(rig::tool::ToolOutput::text(report))
    }
}

/// Arguments for `eval`.
#[derive(Debug, serde::Deserialize)]
pub struct EvalArgs {
    pub cells: Vec<EvalCell>,
}

/// One code cell of the eval run.
#[derive(Debug, serde::Deserialize)]
pub struct EvalCell {
    pub language: String,
    pub code: String,
    pub title: Option<String>,
    pub timeout: Option<u64>,
    /// Requested state reset. No persistent kernel exists yet.
    pub reset: Option<bool>,
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
async fn run_cell(language: &str, code: &str, seconds: u64) -> Result<Captured, String> {
    match language {
        "py" => {
            let arguments = vec!["-c".to_owned(), code.to_owned()];
            capture("python3", &arguments, seconds).await
        },
        "js" => {
            let arguments = vec!["-e".to_owned(), code.to_owned()];
            capture("bun", &arguments, seconds).await
        },
        other => Err(format!("language '{other}' is not supported. Use py or js.")),
    }
}

/// Spawn one program with a watchdog timeout and capture its output.
async fn capture(program: &str, arguments: &[String], seconds: u64) -> Result<Captured, String> {
    let mut command = tokio::process::Command::new(program);
    command.args(arguments).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| format!("{program} failed to start: {error}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let capture = tokio::time::timeout(
        Duration::from_secs(seconds),
        pin!(async {
            let (out_bytes, err_bytes, status) = tokio::join!(drain(stdout), drain(stderr), child.wait());
            (out_bytes, err_bytes, status)
        }),
    )
    .await;
    let (stdout_bytes, stderr_bytes, status) = match capture {
        Ok(parts) => parts,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(format!("{program} timed out after {seconds} seconds"));
        },
    };
    let code = match status {
        Ok(status) => status.code().unwrap_or(-1),
        Err(error) => {
            return Err(format!("{program} status failed: {error}"));
        },
    };
    let mut output = String::from_utf8_lossy(&stdout_bytes).into_owned();
    output.push_str(&String::from_utf8_lossy(&stderr_bytes));
    Ok(Captured {
        code,
        output,
    })
}

/// Read one pipe to the end and return its bytes.
async fn drain<R>(pipe: Option<R>) -> Vec<u8>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut pipe = match pipe {
        Some(pipe) => pipe,
        None => return Vec::new(),
    };
    let mut bytes = Vec::new();
    let _ = tokio::io::AsyncReadExt::read_to_end(&mut pipe, &mut bytes).await;
    bytes
}

/// Cut output at 50 KiB and mark the cut.
fn truncate_output(output: &str) -> String {
    if output.len() <= OUTPUT_LIMIT {
        return output.to_owned();
    }
    let cut = String::from_utf8_lossy(&output.as_bytes()[..OUTPUT_LIMIT]).into_owned();
    format!("{cut}\n[output truncated at 50 KiB]")
}

/// Captured result of one process run.
struct Captured {
    code: i32,
    output: String,
}
