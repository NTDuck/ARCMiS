//! `task` queues task definitions for background subagent work.
//!
//! This pass validates the task list and records each entry as a running job
//! in the shared [`crate::util::jobs::JobRegistry`]. Actual subagent
//! execution lands when the runtime grows a spawner. Until then no job ever
//! completes, and the job tool keeps reporting it as running.

use rig::tool::{Tool, ToolContext, ToolExecutionError, ToolOutput};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

/// `task` validates and queues task definitions as registry jobs.
pub struct Task {
    /// Shared job registry. The task tool records each queued task here.
    pub jobs: Arc<Mutex<crate::util::jobs::JobRegistry>>,
}

impl Tool for Task {
    const NAME: &'static str = "task";
    type Error = ToolExecutionError;
    type Args = TaskArgs;
    type Output = ToolOutput;

    fn description(&self) -> String {
        "Validate task definitions and queue them as background jobs.".to_owned()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "description": "Task definitions to queue",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Unique task id" },
                            "description": { "type": "string", "description": "Short task description" },
                            "assignment": { "type": "string", "description": "Full assignment text" }
                        },
                        "required": ["id", "assignment"]
                    }
                },
                "agent": { "type": "string", "description": "Optional agent type for all tasks" },
                "context": { "type": "string", "description": "Shared context text for all tasks" },
                "schema": { "type": "object", "description": "Optional output schema" },
                "isolated": { "type": "boolean", "description": "Run each task in an isolated worktree" }
            },
            "required": ["tasks"]
        })
    }

    async fn call(&self, _context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let summary = queue_tasks(&self.jobs, &args)?;
        Ok(ToolOutput::text(summary))
    }
}

/// Arguments for `task`.
#[derive(Debug, Deserialize)]
pub struct TaskArgs {
    pub tasks: Vec<TaskDefinition>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub schema: Option<serde_json::Value>,
    #[serde(default)]
    pub isolated: Option<bool>,
}

/// One queued task definition.
#[derive(Debug, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    pub assignment: String,
}

/// Validate the task list, register each entry, and build the summary text.
fn queue_tasks(jobs: &Mutex<crate::util::jobs::JobRegistry>, args: &TaskArgs) -> Result<String, ToolExecutionError> {
    validate(&args.tasks)?;
    let mut registry = lock_registry(jobs)?;
    let ids = args
        .tasks
        .iter()
        .map(|task| {
            let label = task.description.as_deref().unwrap_or(task.id.as_str());
            let job_id = registry.register("task", label);
            format!("{} ({job_id})", task.id)
        })
        .collect::<Vec<_>>();
    let count = ids.len();
    Ok(format!("queued {count} task(s)\n{}", ids.join("\n")))
}

/// Reject empty assignments and duplicate task ids.
fn validate(tasks: &[TaskDefinition]) -> Result<(), ToolExecutionError> {
    if tasks.is_empty() {
        return Err(ToolExecutionError::invalid_args("tasks must hold at least one entry"));
    }
    let mut seen = BTreeSet::new();
    for task in tasks {
        if task.id.is_empty() {
            return Err(ToolExecutionError::invalid_args("task id must not be empty"));
        }
        if task.assignment.trim().is_empty() {
            return Err(ToolExecutionError::invalid_args(format!("task \"{}\" needs a non-empty assignment", task.id)));
        }
        if !seen.insert(task.id.clone()) {
            return Err(ToolExecutionError::invalid_args(format!("duplicate task id \"{}\"", task.id)));
        }
    }
    Ok(())
}

/// Lock the job registry. A poisoned lock returns an execution error.
fn lock_registry(
    jobs: &Mutex<crate::util::jobs::JobRegistry>,
) -> Result<MutexGuard<'_, crate::util::jobs::JobRegistry>, ToolExecutionError> {
    jobs.lock().map_err(|error| ToolExecutionError::other(format!("job registry lock failed: {error}")))
}
