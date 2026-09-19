//! `job` polls, cancels, and lists background jobs.
//!
//! The tool delegates to the shared [`crate::util::jobs::JobRegistry`] on its
//! struct. Poll returns job snapshots grouped by status. Cancel stops running
//! jobs. Every op returns text sections, and async disabled is a text error,
//! not a thrown error.

use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};
use serde::Deserialize;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

/// `job` polls, cancels, and lists registry jobs.
pub struct Job {
    /// Shared job registry.
    pub jobs: Arc<Mutex<crate::util::jobs::JobRegistry>>,
}

impl Tool for Job {
    const NAME: &'static str = "job";
    type Error = ToolExecutionError;
    type Args = JobArgs;
    type Output = ToolOutput;

    fn description(&self) -> String {
        "Poll, cancel, or list background jobs in the registry.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "poll": { "type": "array", "items": { "type": "string" }, "description": "Job ids to poll. Empty polls every job." },
                "cancel": { "type": "array", "items": { "type": "string" }, "description": "Job ids to cancel" },
                "list": { "type": "boolean", "description": "List every job" }
            }
        })
    }

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let text = run_ops(&self.jobs, &args)?;
        Ok(ToolOutput::text(text))
    }
}

/// Arguments for `job`.
#[derive(Debug, Deserialize)]
pub struct JobArgs {
    #[serde(default)]
    pub poll: Option<Vec<String>>,
    #[serde(default)]
    pub cancel: Option<Vec<String>>,
    #[serde(default)]
    pub list: Option<bool>,
}

/// Run the requested ops in poll, cancel, list order. Build the text report.
fn run_ops(jobs: &Mutex<crate::util::jobs::JobRegistry>, args: &JobArgs) -> Result<String, ToolExecutionError> {
    let mut registry = lock_registry(jobs)?;
    registry.retain();
    let mut sections = Vec::new();
    if let Some(cancel_ids) = &args.cancel {
        let cancelled = registry.cancel(cancel_ids);
        sections.push(render_cancelled(&cancelled));
    }
    if let Some(poll_ids) = &args.poll {
        sections.push(render_poll(&registry.poll(poll_ids)));
    } else if args.list.unwrap_or(false) {
        sections.push(render_poll(&registry.poll(&[])));
    }
    if sections.is_empty() {
        sections.push("no op given. Pass \"poll\", \"cancel\", or \"list\": true.".to_owned());
    }
    Ok(sections.join("\n\n"))
}

/// Render the cancelled-id section.
fn render_cancelled(cancelled: &[String]) -> String {
    if cancelled.is_empty() {
        return "## Cancelled\nnone".to_owned();
    }
    format!("## Cancelled\n{}", cancelled.join("\n"))
}

/// Render the poll snapshot grouped by status.
fn render_poll(entries: &[crate::util::jobs::JobEntry]) -> String {
    let mut completed = Vec::new();
    let mut failed = Vec::new();
    let mut running = Vec::new();
    for entry in entries {
        let row = render_entry(entry);
        match entry.status {
            crate::util::jobs::JobStatus::Completed => completed.push(row),
            crate::util::jobs::JobStatus::Failed => failed.push(row),
            _ => running.push(row),
        }
    }
    let mut output = format!("## Completed\n{}", render_rows(&completed));
    if !failed.is_empty() {
        output.push_str(&format!("\n\n## Failed\n{}", render_rows(&failed)));
    }
    output.push_str(&format!("\n\n## Still Running\n{}", render_rows(&running)));
    output
}

/// Render rows, or the word "none" when the section holds no rows.
fn render_rows(rows: &[String]) -> String {
    if rows.is_empty() {
        return "none".to_owned();
    }
    rows.join("\n")
}

/// Render one job row with id, type, status, and captured text.
fn render_entry(entry: &crate::util::jobs::JobEntry) -> String {
    let detail = entry.result_text.as_deref().or(entry.error_text.as_deref()).unwrap_or_default();
    if detail.is_empty() {
        return format!("- {} [{}] {} ({})", entry.id, entry.job_type, entry.label, entry.status.as_str());
    }
    format!("- {} [{}] {} ({}): {detail}", entry.id, entry.job_type, entry.label, entry.status.as_str())
}

/// Lock the job registry. A poisoned lock returns an execution error.
fn lock_registry(
    jobs: &Mutex<crate::util::jobs::JobRegistry>,
) -> Result<MutexGuard<'_, crate::util::jobs::JobRegistry>, ToolExecutionError> {
    jobs.lock().map_err(|error| ToolExecutionError::other(format!("job registry lock failed: {error}")))
}
