//! Job registry for background jobs.
//!
//! A tool records a job when it starts work. The job tool polls the registry,
//! cancels running jobs, and prunes finished entries. Each job stores a type
//! label, a status, and the result or error text captured at completion.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

/// Status of one registered job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    /// The job is running.
    Running,
    /// The job finished with success.
    Completed,
    /// The job finished with failure.
    Failed,
    /// The job stopped through an explicit cancel call before completion.
    Cancelled,
}

impl JobStatus {
    /// Stable lowercase name used in poll output.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// One registered job with its lifecycle data.
#[derive(Debug, Clone)]
pub struct JobEntry {
    /// Job id (uuid string).
    pub id: String,
    /// Job type, for example "task" or "command".
    pub job_type: String,
    /// Human label shown in listings.
    pub label: String,
    /// Current status.
    pub status: JobStatus,
    /// Result text captured on completion.
    pub result_text: Option<String>,
    /// Error text captured on failure.
    pub error_text: Option<String>,
    /// Unix time of the last status change, in seconds.
    pub updated_at: u64,
}

/// Registry of background jobs, shared by the task and job tools.
#[derive(Debug, Default)]
pub struct JobRegistry {
    jobs: BTreeMap<String, JobEntry>,
}

impl JobRegistry {
    /// Create an empty shared registry.
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            jobs: BTreeMap::new(),
        })
    }

    /// Register a running job and return its uuid id.
    pub fn register(&mut self, job_type: &str, label: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.jobs.insert(
            id.clone(),
            JobEntry {
                id: id.clone(),
                job_type: String::from(job_type),
                label: String::from(label),
                status: JobStatus::Running,
                result_text: None,
                error_text: None,
                updated_at: now_seconds(),
            },
        );
        id
    }

    /// Mark a job completed with result text. The registry skips unknown ids.
    pub fn complete(&mut self, id: &str, result_text: &str) {
        self.update(id, |entry| {
            entry.status = JobStatus::Completed;
            entry.result_text = Some(String::from(result_text));
        });
    }

    /// Mark a job failed with error text. The registry skips unknown ids.
    pub fn fail(&mut self, id: &str, error_text: &str) {
        self.update(id, |entry| {
            entry.status = JobStatus::Failed;
            entry.error_text = Some(String::from(error_text));
        });
    }

    /// Snapshot the requested jobs. Empty ids returns every job.
    #[must_use]
    pub fn poll(&self, ids: &[String]) -> Vec<JobEntry> {
        if ids.is_empty() {
            return self.jobs.values().cloned().collect::<Vec<_>>();
        }
        ids.iter().filter_map(|id| self.jobs.get(id)).cloned().collect::<Vec<_>>()
    }

    /// Cancel the requested running jobs. Return the ids that were running.
    pub fn cancel(&mut self, ids: &[String]) -> Vec<String> {
        ids.iter()
            .filter_map(|id| {
                let entry = self.jobs.get_mut(id)?;
                if entry.status == JobStatus::Running {
                    entry.status = JobStatus::Cancelled;
                    entry.updated_at = now_seconds();
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }

    /// Drop finished entries older than 5 minutes. Return the dropped count.
    pub fn retain(&mut self) -> usize {
        let cutoff = now_seconds().saturating_sub(300);
        let stale =
            self.jobs.iter().filter(|(_, entry)| is_stale(entry, cutoff)).map(|(id, _)| id.clone()).collect::<Vec<_>>();
        let dropped = stale.len();
        for id in stale {
            self.jobs.remove(&id);
        }
        dropped
    }
}

/// True when a finished entry updated before `cutoff`.
fn is_stale(entry: &JobEntry, cutoff: u64) -> bool {
    match entry.status {
        JobStatus::Running => false,
        JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled => entry.updated_at < cutoff,
    }
}

/// Apply `change` to one job. The registry skips unknown ids.
impl JobRegistry {
    fn update(&mut self, id: &str, change: impl FnOnce(&mut JobEntry)) {
        let entry = self.jobs.get_mut(id);
        if let Some(entry) = entry {
            change(entry);
            entry.updated_at = now_seconds();
        }
    }
}

/// Current unix time in seconds.
fn now_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |span| span.as_secs())
}
