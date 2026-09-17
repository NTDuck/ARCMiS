//! `task` queues task definitions for background subagent work.
//!
//! This pass validates the task list and records each entry as a running job
//! in the shared [`crate::util::jobs::JobRegistry`]. Actual subagent
//! execution lands when the runtime grows a spawner. Until then no job ever
//! completes, and the job tool keeps reporting it as running.

/// `task` validates and queues task definitions as registry jobs.
pub struct Task {
    /// Shared job registry. The task tool records each queued task here.
    pub jobs: ::std::sync::Arc<::std::sync::Mutex<crate::util::jobs::JobRegistry>>,
}

impl ::rig::tool::Tool for Task {
    const NAME: &'static str = "task";
    type Error = ::rig::tool::ToolExecutionError;
    type Args = TaskArgs;
    type Output = ::rig::tool::ToolOutput;

    fn description(&self) -> ::std::string::String {
        "Validate task definitions and queue them as background jobs.".to_owned()
    }

    fn parameters(&self) -> ::serde_json::Value {
        ::serde_json::json!({
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

    async fn call(
        &self,
        _context: &mut ::rig::tool::ToolContext,
        args: Self::Args,
    ) -> ::core::result::Result<Self::Output, Self::Error> {
        let summary = queue_tasks(&self.jobs, &args)?;
        ::core::result::Result::Ok(::rig::tool::ToolOutput::text(summary))
    }
}

/// Arguments for `task`.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
pub struct TaskArgs {
    pub tasks: ::std::vec::Vec<TaskDefinition>,
    #[serde(default)]
    pub agent: ::core::option::Option<::std::string::String>,
    #[serde(default)]
    pub context: ::core::option::Option<::std::string::String>,
    #[serde(default)]
    pub schema: ::core::option::Option<::serde_json::Value>,
    #[serde(default)]
    pub isolated: ::core::option::Option<bool>,
}

/// One queued task definition.
#[derive(::core::fmt::Debug, ::serde::Deserialize)]
pub struct TaskDefinition {
    pub id: ::std::string::String,
    #[serde(default)]
    pub description: ::core::option::Option<::std::string::String>,
    pub assignment: ::std::string::String,
}

/// Validate the task list, register each entry, and build the summary text.
fn queue_tasks(
    jobs: &::std::sync::Mutex<crate::util::jobs::JobRegistry>,
    args: &TaskArgs,
) -> ::core::result::Result<::std::string::String, ::rig::tool::ToolExecutionError> {
    validate(&args.tasks)?;
    let mut registry = lock_registry(jobs)?;
    let ids = args
        .tasks
        .iter()
        .map(|task| {
            let label = task.description.as_deref().unwrap_or(task.id.as_str());
            let job_id = registry.register("task", label);
            ::std::format!("{} ({job_id})", task.id)
        })
        .collect::<::std::vec::Vec<_>>();
    let count = ids.len();
    ::core::result::Result::Ok(::std::format!("queued {count} task(s)\n{}", ids.join("\n")))
}

/// Reject empty assignments and duplicate task ids.
fn validate(tasks: &[TaskDefinition]) -> ::core::result::Result<(), ::rig::tool::ToolExecutionError> {
    if tasks.is_empty() {
        return ::core::result::Result::Err(::rig::tool::ToolExecutionError::invalid_args(
            "tasks must hold at least one entry",
        ));
    }
    let mut seen = ::std::collections::BTreeSet::new();
    for task in tasks {
        if task.id.is_empty() {
            return ::core::result::Result::Err(::rig::tool::ToolExecutionError::invalid_args(
                "task id must not be empty",
            ));
        }
        if task.assignment.trim().is_empty() {
            return ::core::result::Result::Err(::rig::tool::ToolExecutionError::invalid_args(::std::format!(
                "task \"{}\" needs a non-empty assignment",
                task.id
            )));
        }
        if !seen.insert(::std::clone::Clone::clone(&task.id)) {
            return ::core::result::Result::Err(::rig::tool::ToolExecutionError::invalid_args(::std::format!(
                "duplicate task id \"{}\"",
                task.id
            )));
        }
    }
    ::core::result::Result::Ok(())
}

/// Lock the job registry. A poisoned lock returns an execution error.
fn lock_registry(
    jobs: &::std::sync::Mutex<crate::util::jobs::JobRegistry>,
) -> ::core::result::Result<::std::sync::MutexGuard<'_, crate::util::jobs::JobRegistry>, ::rig::tool::ToolExecutionError>
{
    jobs.lock()
        .map_err(|error| ::rig::tool::ToolExecutionError::other(::std::format!("job registry lock failed: {error}")))
}
