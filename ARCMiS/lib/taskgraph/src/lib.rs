//! `mas_taskgraph` is the typed task DAG with fan-out bounds, per-node repair
//! budgets, and stagnation detection!
//!
//! WHY a plain `Vec` adjacency over `petgraph`: the migration phase
//! structure is shallow, cycles are absent by construction (dependencies
//! point backward only), and every traversal is a readiness scan. The
//! dependency adds nothing until proven otherwise (`minimal-code.md`).

// The workspace naming rule prefixes crates with ARCMiS-. The generated
// snake_case identifier trips only this lint.
#![allow(non_snake_case)]

use std::path::PathBuf;

use hierarchy::Role;
use phase::Evidence;
use phase::Phase;
use verdict::Verdict;

/// Index-based node handle.
pub type NodeId = usize;

/// Status of one work item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// Waiting for dependencies.
    Queued,
    /// Accepted under the fan-out bound, waiting for admission.
    Admitted,
    /// Running.
    InProgress,
    /// Finished successfully.
    Done,
    /// Terminally failed: budget spent on repairs.
    Failed,
}

/// The typed workload a node carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// Source-code analysis toward the source map.
    Analyze,
    /// Dependency planning into batches.
    Plan,
    /// Translate one batch.
    Translate,
    /// Judge one translated batch.
    Validate,
    /// Run the test command for one batch.
    Test,
    /// Repair one batch after diagnosed failure.
    Repair,
    /// Compose translated batches.
    Integrate,
    /// End-of-run adversarial review.
    Review,
}

/// One unit of work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItem {
    /// Specialist meant to pick the item up.
    pub role: Role,
    /// The node's typed workload.
    pub kind: Kind,
    /// Instruction text handed to the specialist.
    pub instruction: String,
}

/// Unique identifier for one task contract. Format: `<phase>-<unit>-<index>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(pub String);

/// Migration strategy for one task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Strategy {
    /// Lift the source code with minimal change.
    Uplift,
    /// Transform the source code into idiomatic target code.
    Transform,
    /// Reimagine the module from its specification.
    Reimagine,
}

/// Risk level for one task. Computed from the unit's complexity and the
/// strategy's risk profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Risk {
    /// Low risk: routine translation.
    Low,
    /// Medium risk: structural change required.
    Medium,
    /// High risk: redesign or security-sensitive change.
    High,
}

/// Reference to one required input artifact. Carries the content hash
/// for provenance verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputRef {
    /// Artifact id in the store.
    pub artifact_id: String,
    /// Content hash at the time the input was bound.
    pub content_hash: String,
    /// Why the task needs this input.
    pub purpose: String,
}

/// Paths the task may write. The gateway enforces the scope.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteScope {
    /// Paths the task may write.
    pub allowed_paths: Vec<PathBuf>,
    /// Paths the task may never write, even when inside an allowed path.
    pub denied_paths: Vec<PathBuf>,
}

/// Typed task contract (ADR 0040 section 1). The `WorkItem` grows into
/// this struct while the old fields stay for backward compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskContract {
    /// Unique identifier.
    pub task_id: TaskId,
    /// Run-level phase that owns the task.
    pub phase: Phase,
    /// Module or batch the task operates on.
    pub unit: String,
    /// Migration strategy.
    pub strategy: Strategy,
    /// Risk level.
    pub risk: Risk,
    /// Specialist role that must execute the task.
    pub required_role: Role,
    /// Required input artifacts with content hashes.
    pub required_inputs: Vec<InputRef>,
    /// Paths the task may write.
    pub write_scope: WriteScope,
    /// Maximum attempts before escalation.
    pub max_attempts: usize,
    /// Whether the task requires verification before completion.
    pub verification_required: bool,
}

impl TaskContract {
    /// Default maximum attempts before escalation.
    pub const DEFAULT_MAX_ATTEMPTS: usize = 2;
}

/// Failure class for one task result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// The translation step failed.
    TranslationError,
    /// The verification step failed.
    VerificationError,
    /// The task ran past its time budget.
    TimeoutError,
    /// The task exhausted its attempt budget.
    BudgetExhausted,
    /// The task was escalated to a human.
    Escalated,
}

/// Outcome of one task execution.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskResult {
    /// Task that produced the result.
    pub task_id: TaskId,
    /// Verdict for the task.
    pub verdict: Verdict,
    /// Artifacts the task produced.
    pub artifacts: Vec<String>,
    /// Evidence the task collected.
    pub evidence: Evidence,
    /// Attempts the task consumed.
    pub attempts_used: usize,
    /// Failure class when the task failed.
    pub failure_class: Option<FailureClass>,
}

impl From<WorkItem> for TaskContract {
    fn from(item: WorkItem) -> Self {
        let strategy = match item.kind {
            Kind::Analyze | Kind::Plan | Kind::Review => Strategy::Uplift,
            Kind::Translate | Kind::Integrate => Strategy::Transform,
            Kind::Validate | Kind::Test | Kind::Repair => Strategy::Reimagine,
        };
        Self {
            task_id: TaskId(format!("legacy-{}-{}", item.role.name(), item.instruction.len())),
            phase: Phase::Migration,
            unit: item.instruction.clone(),
            strategy,
            risk: Risk::Medium,
            required_role: item.role,
            required_inputs: Vec::new(),
            write_scope: WriteScope::default(),
            max_attempts: Self::DEFAULT_MAX_ATTEMPTS,
            verification_required: true,
        }
    }
}

/// One graph node: the work plus its execution state.
#[derive(Debug)]
struct Node {
    item: WorkItem,
    dependencies: Vec<NodeId>,
    status: TaskStatus,
    repairs_used: usize,
}

impl Node {
    fn ready(&self) -> bool {
        self.status == TaskStatus::Queued
    }
}

/// Ceilings and bounds from the run config's `limits` section. Each one
/// bubbles to the call site: zero disables the bound it names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GraphBudgets {
    /// Maximum agents dispatched concurrently per round (ADR 0022).
    pub fanout: usize,
    /// Repairs per work item before escalation.
    pub max_repairs: usize,
    /// Consecutive stagnant rounds before the loop escalates.
    pub stagnation_rounds: usize,
}

/// Node-count snapshot by status, for gate evidence and reporting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StatusCounts {
    /// Nodes waiting on dependencies.
    pub queued: usize,
    /// Nodes holding a fan-out slot.
    pub admitted: usize,
    /// Nodes running.
    pub in_progress: usize,
    /// Nodes finished.
    pub done: usize,
    /// Nodes terminally failed.
    pub failed: usize,
}

/// Directed acyclic work graph. Dependencies must point backward only:
/// `add` inserts nodes so each edge references an item already present,
/// which excludes cycles by construction.
#[derive(Debug)]
pub struct TaskGraph {
    nodes: Vec<Node>,
    budgets: GraphBudgets,
    /// Consecutive rounds with no completion. `None` until the first
    /// stalled round so a fresh graph does not trip instantly.
    stagnant_rounds: usize,
}

impl TaskGraph {
    /// Empty graph under the caller's budgets.
    pub fn new(budgets: GraphBudgets) -> Self {
        Self {
            nodes: Vec::new(),
            budgets,
            stagnant_rounds: 0,
        }
    }

    /// Node count.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the graph holds no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The bounds governing this graph.
    pub fn budgets(&self) -> GraphBudgets {
        self.budgets
    }

    /// The node count by status, for progress reporting.
    pub fn status(&self, id: NodeId) -> TaskStatus {
        self.nodes[id].status
    }

    /// Insert a node with its dependencies. Panics on a forward edge.
    pub fn add(&mut self, item: WorkItem, dependencies: Vec<NodeId>) -> NodeId {
        let id = self.nodes.len();
        assert!(dependencies.iter().all(|dependency| dependency < &id), "task {id} has a forward dependency");
        self.nodes.push(Node {
            item,
            dependencies,
            status: TaskStatus::Queued,
            repairs_used: 0,
        });
        id
    }

    /// Queued nodes whose dependencies are all `Done`, ascending by id.
    /// Reserves one `Admitted` slot each for dependencies held by no
    /// queued node.
    pub fn ready(&self) -> Vec<NodeId> {
        let mut candidates = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(id, node)| node.ready() && self.all_dependencies_done(*id))
            .map(|(id, _)| id)
            .collect::<Vec<NodeId>>();
        let slack = self.slots_remaining();
        candidates.truncate(slack.unwrap_or(candidates.len()));
        candidates
    }

    /// Admission slots open: `fanout` minus busy nodes, or `None` for
    /// an unrestricted graph.
    fn slots_remaining(&self) -> Option<usize> {
        if self.budgets.fanout == 0 {
            return None;
        }
        let busy = self
            .nodes
            .iter()
            .filter(|node| node.status == TaskStatus::Admitted || node.status == TaskStatus::InProgress)
            .count();
        Some(self.budgets.fanout.saturating_sub(busy))
    }

    fn all_dependencies_done(&self, id: NodeId) -> bool {
        self.nodes[id].dependencies.iter().all(|dependency| self.nodes[*dependency].status == TaskStatus::Done)
    }

    /// Reserve one slot. `Queued` becomes `Admitted`. No-op otherwise.
    pub fn admit(&mut self, id: NodeId) {
        let node = &mut self.nodes[id];
        if node.ready() {
            node.status = TaskStatus::Admitted;
        }
    }

    /// Begin admitted work: `Admitted` becomes `InProgress`.
    pub fn start(&mut self, id: NodeId) {
        if self.nodes[id].status == TaskStatus::Admitted {
            self.nodes[id].status = TaskStatus::InProgress;
        }
    }

    /// Record one successful completion. Clears the stagnation counter.
    pub fn finish(&mut self, id: NodeId) {
        self.nodes[id].status = TaskStatus::Done;
        self.record_progress();
    }

    /// Record one failure. While repair budget remains, the node returns
    /// to `Queued` for a retry. After budget exhausts it is `Failed`.
    pub fn record_failure(&mut self, id: NodeId) -> TaskStatus {
        let node = &mut self.nodes[id];
        if node.repairs_used < self.budgets.max_repairs {
            node.repairs_used += 1;
            node.status = TaskStatus::Queued;
        } else {
            node.status = TaskStatus::Failed;
        }
        node.status
    }

    /// Repairs consumed by one node.
    pub fn repairs_used(&self, id: NodeId) -> usize {
        self.nodes[id].repairs_used
    }

    /// The work item behind one node.
    pub fn item(&self, id: NodeId) -> &WorkItem {
        &self.nodes[id].item
    }

    /// Node ids currently `InProgress`, ascending.
    pub fn ids_in_progress(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.status == TaskStatus::InProgress)
            .map(|(id, _)| id)
            .collect()
    }

    /// Status tallies for progress and gate evidence.
    pub fn status_counts(&self) -> StatusCounts {
        let mut counts = StatusCounts::default();
        for node in &self.nodes {
            match node.status {
                TaskStatus::Queued => counts.queued += 1,
                TaskStatus::Admitted => counts.admitted += 1,
                TaskStatus::InProgress => counts.in_progress += 1,
                TaskStatus::Done => counts.done += 1,
                TaskStatus::Failed => counts.failed += 1,
            }
        }
        counts
    }

    /// Declare the current round productive: reset stagnation.
    pub fn record_progress(&mut self) {
        self.stagnant_rounds = 0;
    }

    /// Declare the current round stagnant. Returns `true` when the count
    /// reaches the configured ceiling.
    pub fn record_stagnation(&mut self) -> bool {
        self.stagnant_rounds += 1;
        self.budgets.stagnation_rounds > 0 && self.stagnant_rounds >= self.budgets.stagnation_rounds
    }

    /// Nodes currently counted as busy under the fan-out bound.
    pub fn running_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.status == TaskStatus::Admitted || node.status == TaskStatus::InProgress)
            .count()
    }

    /// Whether every node reached `Done` or `Failed`: nothing queued,
    /// admitted, or running.
    pub fn is_settled(&self) -> bool {
        self.nodes.iter().all(|node| node.status == TaskStatus::Done || node.status == TaskStatus::Failed)
    }
}
