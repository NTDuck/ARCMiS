//! Integration tests for the task graph: readiness, fanout, repairs,
//! stagnation.

use mas_core::hierarchy::Role;
use mas_core::taskgraph::GraphBudgets;
use mas_core::taskgraph::TaskGraph;
use mas_core::taskgraph::TaskStatus;
use mas_core::taskgraph::WorkItem;

fn item(role: Role, instruction: &str) -> WorkItem {
    WorkItem {
        role,
        kind: mas_core::taskgraph::Kind::Translate,
        instruction: instruction.to_owned(),
    }
}

fn budgets() -> GraphBudgets {
    GraphBudgets {
        fanout: 2,
        max_repairs: 2,
        stagnation_rounds: 3,
    }
}

#[test]
fn forward_edges_are_rejected() {
    let mut graph = TaskGraph::new(budgets());
    graph.add(item(Role::Planner, "plan"), Vec::new());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        graph.add(item(Role::Translator, "depends"), vec![1])
    }));
    assert!(result.is_err());
}

#[test]
fn readiness_respects_dependencies() {
    let mut graph = TaskGraph::new(budgets());
    let parent = graph.add(item(Role::Planner, "plan"), Vec::new());
    let child = graph.add(item(Role::Translator, "translate"), vec![parent]);
    assert_eq!(graph.ready(), vec![parent]);
    graph.finish(parent);
    assert_eq!(graph.ready(), vec![child]);
}

#[test]
fn fanout_bounds_admission() {
    let mut graph = TaskGraph::new(budgets());
    let first = graph.add(item(Role::Translator, "one"), Vec::new());
    let second = graph.add(item(Role::Translator, "two"), Vec::new());
    let third = graph.add(item(Role::Translator, "three"), Vec::new());
    assert_eq!(graph.ready(), vec![first, second]);
    graph.admit(first);
    graph.admit(second);
    assert!(graph.ready().is_empty());
    graph.start(first);
    graph.finish(first);
    assert_eq!(graph.ready(), vec![third]);
    assert_eq!(graph.running_count(), 1);
}

#[test]
fn zero_fanout_is_unlimited() {
    let limits = GraphBudgets {
        fanout: 0,
        max_repairs: 1,
        stagnation_rounds: 0,
    };
    let mut graph = TaskGraph::new(limits);
    graph.add(item(Role::Translator, "one"), Vec::new());
    graph.add(item(Role::Translator, "two"), Vec::new());
    graph.add(item(Role::Translator, "three"), Vec::new());
    assert_eq!(graph.ready().len(), 3);
}

#[test]
fn repairs_burn_before_terminal_failure() {
    let mut graph = TaskGraph::new(budgets());
    let id = graph.add(item(Role::Translator, "flaky"), Vec::new());
    graph.admit(id);
    assert_eq!(graph.record_failure(id), TaskStatus::Queued);
    assert_eq!(graph.repairs_used(id), 1);
    assert_eq!(graph.record_failure(id), TaskStatus::Queued);
    assert_eq!(graph.record_failure(id), TaskStatus::Failed);
    assert_eq!(graph.status(id), TaskStatus::Failed);
}

#[test]
fn stagnation_counter_trips_at_ceiling() {
    let mut graph = TaskGraph::new(budgets());
    assert!(!graph.record_stagnation());
    assert!(!graph.record_stagnation());
    assert!(graph.record_stagnation());
    graph.record_progress();
    assert!(!graph.record_stagnation());
}
