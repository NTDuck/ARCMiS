//! Integration tests for the taskgraph crate.

use hierarchy::Role;
use mas_taskgraph::GraphBudgets;
use mas_taskgraph::Kind;
use mas_taskgraph::TaskGraph;
use mas_taskgraph::TaskStatus;
use mas_taskgraph::WorkItem;

fn item(role: Role, instruction: &str) -> WorkItem {
    WorkItem {
        role,
        kind: Kind::Translate,
        instruction: instruction.to_owned(),
    }
}

#[test]
fn empty_graph_is_settled() {
    let graph = TaskGraph::new(GraphBudgets::default());
    assert!(graph.is_empty());
    assert!(graph.is_settled());
}

#[test]
fn readiness_respects_dependencies() {
    let mut graph = TaskGraph::new(GraphBudgets::default());
    let first = graph.add(item(Role::UpliftWorker, "first"), vec![]);
    let second = graph.add(item(Role::UpliftWorker, "second"), vec![first]);
    assert_eq!(graph.ready(), vec![first]);
    graph.admit(first);
    graph.start(first);
    graph.finish(first);
    assert_eq!(graph.ready(), vec![second]);
}

#[test]
fn forward_edges_are_rejected() {
    let mut graph = TaskGraph::new(GraphBudgets::default());
    graph.add(item(Role::UpliftWorker, "first"), vec![]);
    let result = std::panic::catch_unwind(move || {
        let mut graph = graph;
        graph.add(item(Role::UpliftWorker, "second"), vec![5]);
    });
    assert!(result.is_err());
}

#[test]
fn fanout_bounds_admission() {
    let mut graph = TaskGraph::new(GraphBudgets {
        fanout: 1,
        max_repairs: 0,
        stagnation_rounds: 0,
    });
    graph.add(item(Role::UpliftWorker, "first"), vec![]);
    graph.add(item(Role::UpliftWorker, "second"), vec![]);
    let ready = graph.ready();
    assert_eq!(ready.len(), 1);
}

#[test]
fn zero_fanout_is_unlimited() {
    let mut graph = TaskGraph::new(GraphBudgets::default());
    graph.add(item(Role::UpliftWorker, "first"), vec![]);
    graph.add(item(Role::UpliftWorker, "second"), vec![]);
    graph.add(item(Role::UpliftWorker, "third"), vec![]);
    assert_eq!(graph.ready().len(), 3);
}

#[test]
fn repairs_burn_before_terminal_failure() {
    let mut graph = TaskGraph::new(GraphBudgets {
        fanout: 0,
        max_repairs: 2,
        stagnation_rounds: 0,
    });
    let id = graph.add(item(Role::UpliftWorker, "first"), vec![]);
    graph.admit(id);
    graph.start(id);
    assert_eq!(graph.record_failure(id), TaskStatus::Queued);
    assert_eq!(graph.repairs_used(id), 1);
    assert_eq!(graph.record_failure(id), TaskStatus::Queued);
    assert_eq!(graph.repairs_used(id), 2);
    assert_eq!(graph.record_failure(id), TaskStatus::Failed);
}

#[test]
fn stagnation_counter_trips_at_ceiling() {
    let mut graph = TaskGraph::new(GraphBudgets {
        fanout: 0,
        max_repairs: 0,
        stagnation_rounds: 2,
    });
    assert!(!graph.record_stagnation());
    assert!(graph.record_stagnation());
}
