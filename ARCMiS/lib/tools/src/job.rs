//! `job` polls, cancels, and lists background jobs.
//!
//! The tool delegates to the shared [`crate::util::jobs::JobRegistry`] on its
//! struct. Poll returns job snapshots grouped by status. Cancel stops running
//! jobs. Every op returns text sections, and async disabled is a text error,
//! not a thrown error.

/// `job` polls, cancels, and lists registry jobs.
pub struct Job {
    /// Shared job registry.
    pub jobs: std::sync::Arc<std::sync::Mutex<crate::util::jobs::JobRegistry>>,
}

impl rig::tool::Tool for Job {
    const NAME: &'static str = "job";
    type Error = rig::tool::ToolExecutionError;
    type Args = JobArgs;
    type Output = rig::tool::ToolOutput;

    fn description(&self) -> std::string::String {
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

    async fn call(
        &self,
        _context: &mut rig::tool::ToolContext,
        args: Self::Args,
    ) -> core::result::Result<Self::Output, Self::Error> {
        let text = run_ops(&self.jobs, &args)?;
        core::result::Result::Ok(rig::tool::ToolOutput::text(text))
    }
}

/// Arguments for `job`.
#[derive(Debug, serde::Deserialize)]
pub struct JobArgs {
    #[serde(default)]
    pub poll: core::option::Option<std::vec::Vec<std::string::String>>,
    #[serde(default)]
    pub cancel: core::option::Option<std::vec::Vec<std::string::String>>,
    #[serde(default)]
    pub list: core::option::Option<bool>,
}

/// Run the requested ops in poll, cancel, list order. Build the text report.
fn run_ops(
    jobs: &std::sync::Mutex<crate::util::jobs::JobRegistry>,
    args: &JobArgs,
) -> core::result::Result<std::string::String, rig::tool::ToolExecutionError> {
    let mut registry = lock_registry(jobs)?;
    registry.retain();
    let mut sections = std::vec::Vec::new();
    if let core::option::Option::Some(cancel_ids) = &args.cancel {
        let cancelled = registry.cancel(cancel_ids);
        sections.push(render_cancelled(&cancelled));
    }
    if let core::option::Option::Some(poll_ids) = &args.poll {
        sections.push(render_poll(&registry.poll(poll_ids)));
    } else if args.list.unwrap_or(false) {
        sections.push(render_poll(&registry.poll(&[])));
    }
    if sections.is_empty() {
        sections.push(std::string::String::from("no op given. Pass \"poll\", \"cancel\", or \"list\": true."));
    }
    core::result::Result::Ok(sections.join("\n\n"))
}

/// Render the cancelled-id section.
fn render_cancelled(cancelled: &[std::string::String]) -> std::string::String {
    if cancelled.is_empty() {
        return std::string::String::from("## Cancelled\nnone");
    }
    std::format!("## Cancelled\n{}", cancelled.join("\n"))
}

/// Render the poll snapshot grouped by status.
fn render_poll(entries: &[crate::util::jobs::JobEntry]) -> std::string::String {
    let mut completed = std::vec::Vec::new();
    let mut failed = std::vec::Vec::new();
    let mut running = std::vec::Vec::new();
    for entry in entries {
        let row = render_entry(entry);
        match entry.status {
            crate::util::jobs::JobStatus::Completed => completed.push(row),
            crate::util::jobs::JobStatus::Failed => failed.push(row),
            _ => running.push(row),
        }
    }
    let mut output = std::format!("## Completed\n{}", render_rows(&completed));
    if !failed.is_empty() {
        output.push_str(&std::format!("\n\n## Failed\n{}", render_rows(&failed)));
    }
    output.push_str(&std::format!("\n\n## Still Running\n{}", render_rows(&running)));
    output
}

/// Render rows, or the word "none" when the section holds no rows.
fn render_rows(rows: &[std::string::String]) -> std::string::String {
    if rows.is_empty() {
        return std::string::String::from("none");
    }
    rows.join("\n")
}

/// Render one job row with id, type, status, and captured text.
fn render_entry(entry: &crate::util::jobs::JobEntry) -> std::string::String {
    let detail = entry.result_text.as_deref().or(entry.error_text.as_deref()).unwrap_or_default();
    if detail.is_empty() {
        return std::format!("- {} [{}] {} ({})", entry.id, entry.job_type, entry.label, entry.status.as_str());
    }
    std::format!("- {} [{}] {} ({}): {detail}", entry.id, entry.job_type, entry.label, entry.status.as_str())
}

/// Lock the job registry. A poisoned lock returns an execution error.
fn lock_registry(
    jobs: &std::sync::Mutex<crate::util::jobs::JobRegistry>,
) -> core::result::Result<std::sync::MutexGuard<'_, crate::util::jobs::JobRegistry>, rig::tool::ToolExecutionError> {
    jobs.lock().map_err(|error| rig::tool::ToolExecutionError::other(std::format!("job registry lock failed: {error}")))
}
