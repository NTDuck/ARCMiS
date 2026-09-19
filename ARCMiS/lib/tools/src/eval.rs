//! `eval` runs short code cells in Python or JavaScript.
//!
//! Each cell runs in a fresh `python3 -c` or `bun -e` process. A later cell
//! does not see the state of an earlier cell. Persistent per-language kernels
//! are a known deferral and land in a later pass. Each cell has its own
//! timeout in seconds with default 300, clamped to the range 1 through 3600.
//! A failed cell stops the run. Remaining cells report the status `skipped`.

use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};

use crate::util::proc::{capture, clamp_seconds, truncate_output, Captured, ProcError};

/// `eval` runs code cells in Python or JavaScript and reports each result.
pub struct Eval {}

impl Tool for Eval {
    const NAME: &'static str = "eval";
    type Error = ToolExecutionError;
    type Args = EvalArgs;
    type Output = ToolOutput;

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

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.cells.is_empty() {
            return Err(ToolExecutionError::invalid_args("cells must hold at least one cell.".to_owned()));
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
        Ok(ToolOutput::text(report))
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

/// Run one cell in a fresh interpreter process and capture its output.
async fn run_cell(language: &str, code: &str, seconds: u64) -> Result<Captured, ProcError> {
    match language {
        "py" => {
            let arguments = vec!["-c".to_owned(), code.to_owned()];
            capture("python3", &arguments, None, None, seconds).await
        },
        "js" => {
            let arguments = vec!["-e".to_owned(), code.to_owned()];
            capture("bun", &arguments, None, None, seconds).await
        },
        other => Err(ProcError::Spawn(format!("language '{other}' is not supported. Use py or js."))),
    }
}
