//! `tasks.json`: the manager TODO — one ARCMiS run's dynamic task list.
//! Mirrors GVS5H's manager-curated task file; specialists consume it.

use anyhow::Context as _;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::path::PathBuf;

/// One task on the manager list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    /// Stable id (e.g. `t1`, `t2`).
    pub id: String,
    /// What the task asks one specialist to do.
    pub description: String,
    /// Lifecycle status.
    pub status: TaskStatus,
    /// Task ids this one depends on.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Specialist role the manager assigned, when any.
    #[serde(default)]
    pub assigned_to: Option<String>,
}

/// Task lifecycle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Not started.
    Pending,
    /// A specialist is working on it.
    InProgress,
    /// Finished successfully.
    Done,
    /// Blocked on a dependency or an external input.
    Blocked,
}

/// The task list file.
#[derive(Debug, Clone)]
pub struct TaskList {
    path: PathBuf,
}

impl TaskList {
    /// Bind `tasks.json` at `{dir}/tasks.json`.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join("tasks.json"),
        }
    }

    /// Read the list; empty before the first write.
    pub fn read(&self) -> anyhow::Result<Vec<Task>> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(serde_json::from_str(&text)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(error.into()),
        }
    }

    /// Write the whole list.
    pub fn write(&self, tasks: &[Task]) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.path.parent().context("tasks dir")?)?;
        std::fs::write(&self.path, serde_json::to_string_pretty(tasks)?)?;
        Ok(())
    }

    /// Append one task with a generated id; returns the id.
    pub fn add(&self, description: &str, depends_on: &[String]) -> anyhow::Result<String> {
        let mut tasks = self.read()?;
        let id = format!("t{}", tasks.len() + 1);
        tasks.push(Task {
            id: id.clone(),
            description: description.to_owned(),
            status: TaskStatus::Pending,
            depends_on: depends_on.to_vec(),
            assigned_to: None,
        });
        self.write(&tasks)?;
        Ok(id)
    }

    /// Set one task's status.
    pub fn set_status(&self, id: &str, status: TaskStatus) -> anyhow::Result<()> {
        let mut tasks = self.read()?;
        let task = tasks.iter_mut().find(|task| task.id == id).with_context(|| format!("task {id} not found"))?;
        task.status = status;
        self.write(&tasks)
    }

    /// Merge `incoming` into the list, deduping by (description, depends_on).
    /// Returns the ids of tasks that were new.
    pub fn merge_dedup(&self, incoming: Vec<Task>) -> anyhow::Result<Vec<String>> {
        let mut tasks = self.read()?;
        let mut added = Vec::new();
        for task in incoming {
            let duplicate = tasks
                .iter()
                .any(|existing| existing.description == task.description && existing.depends_on == task.depends_on);
            if duplicate {
                continue;
            }
            let id = format!("t{}", tasks.len() + 1);
            added.push(id.clone());
            tasks.push(Task {
                id,
                ..task
            });
        }
        self.write(&tasks)?;
        Ok(added)
    }
}
