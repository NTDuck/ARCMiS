//! Progress tracker: how far the run has come, from the blackboard files.
//! The orchestrator reads this each round; the observability controller reads
//! it after the run.

use blackboard::Phase;
use blackboard::TaskList;
use blackboard::TaskStatus;
use serde::Serialize;

/// One progress snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    /// Current phase.
    pub phase: Phase,
    /// 0..10 position in the forward order.
    pub phase_index: usize,
    /// Total tasks known.
    pub total_tasks: usize,
    /// Tasks in Done status.
    pub done_tasks: usize,
    /// Tasks in InProgress status.
    pub in_progress_tasks: usize,
    /// Tasks in Blocked status.
    pub blocked_tasks: usize,
    /// Batches finished (validated) / total batches, when a plan exists.
    pub batches_done: usize,
    pub batches_total: usize,
}

/// Read the progress from the run directory. Missing files read as empty.
pub fn snapshot(run_dir: &std::path::Path) -> anyhow::Result<Progress> {
    let state = blackboard::state::read(run_dir)?.unwrap_or(blackboard::State {
        phase: Phase::Preflight,
        phase_delegations: 0,
        phase_delegation_watermark: 0,
        current_task: None,
        current_batch: None,
        current_model: String::new(),
        updated_at: String::new(),
        last_transition: String::new(),
    });
    let tasks = TaskList::new(run_dir).read()?;
    let plan = std::fs::read_to_string(run_dir.join("analysis").join("plan.json")).unwrap_or_default();
    let (batches_total, batches_done) = count_batches(&plan, &tasks);
    Ok(Progress {
        phase: state.phase,
        phase_index: crate::state_machine::progress(state.phase),
        total_tasks: tasks.len(),
        done_tasks: tasks.iter().filter(|task| task.status == TaskStatus::Done).count(),
        in_progress_tasks: tasks.iter().filter(|task| task.status == TaskStatus::InProgress).count(),
        blocked_tasks: tasks.iter().filter(|task| task.status == TaskStatus::Blocked).count(),
        batches_done,
        batches_total,
    })
}

/// Count batches from the plan json and the done tasks that reference them.
fn count_batches(plan_json: &str, tasks: &[blackboard::Task]) -> (usize, usize) {
    #[derive(serde::Deserialize)]
    struct Plan {
        #[serde(default)]
        batches: Vec<Batch>,
    }
    #[derive(serde::Deserialize)]
    struct Batch {
        id: String,
    }
    let Ok(plan) = serde_json::from_str::<Plan>(plan_json) else {
        return (0, 0);
    };
    let total = plan.batches.len();
    let done = plan
        .batches
        .iter()
        .filter(|batch| {
            tasks.iter().any(|task| task.status == TaskStatus::Done && task.description.contains(&batch.id))
        })
        .count();
    (done, total)
}
