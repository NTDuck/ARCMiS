//! One graph node: the work plus its execution state.

use crate::node_id::NodeId;
use crate::task_status::TaskStatus;
use crate::work_item::WorkItem;

/// One graph node: the work plus its execution state.
#[derive(Debug)]
pub(crate) struct Node {
    pub(crate) item: WorkItem,
    pub(crate) dependencies: Vec<NodeId>,
    pub(crate) status: TaskStatus,
    pub(crate) repairs_used: usize,
}

impl Node {
    pub(crate) fn ready(&self) -> bool {
        self.status == TaskStatus::Queued
    }
}
