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

pub mod failure_class;
pub mod graph_budgets;
pub mod input_ref;
pub mod kind;
pub mod node;
pub mod node_id;
pub mod risk;
pub mod status_counts;
pub mod strategy;
pub mod task_contract;
pub mod task_id;
pub mod task_result;
pub mod task_status;
pub mod work_item;
pub mod work_item_to_contract;
pub mod write_scope;

use crate::node::Node;

pub use crate::failure_class::FailureClass;
pub use crate::graph_budgets::GraphBudgets;
pub use crate::input_ref::InputRef;
pub use crate::kind::Kind;
pub use crate::node_id::NodeId;
pub use crate::risk::Risk;
pub use crate::status_counts::StatusCounts;
pub use crate::strategy::Strategy;
pub use crate::task_contract::TaskContract;
pub use crate::task_id::TaskId;
pub use crate::task_result::TaskResult;
pub use crate::task_status::TaskStatus;
pub use crate::work_item::WorkItem;
pub use crate::write_scope::WriteScope;

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
